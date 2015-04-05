use std::collections::{HashMap, HashSet, VecMap};
use std::mem;

use physics::{CHUNK_SIZE, TILE_SIZE};

use types::*;
use util::{multimap_insert, multimap_remove};
use util::RefcountedMap;
use util::OptionIterExt;
use util::SmallSet;


pub const VIEW_SIZE: V2 = V2 { x: 5, y: 6 };
pub const VIEW_ANCHOR: V2 = V2 { x: 2, y: 2 };

pub fn vision_region(pos: V3) -> Region<V2> {
    let center = pos.reduce().div_floor(scalar(CHUNK_SIZE * TILE_SIZE));

    let base = center - VIEW_ANCHOR;
    Region::new(base, base + VIEW_SIZE)
}


pub struct Vision {
    clients: VecMap<VisionClient>,
    entities: VecMap<VisionEntity>,
    structures: VecMap<VisionStructure>,

    clients_by_chunk: HashMap<V2, HashSet<ClientId>>,
    entities_by_chunk: HashMap<V2, HashSet<EntityId>>,
    structures_by_chunk: HashMap<V2, HashSet<StructureId>>,

    loaded_chunks: HashSet<V2>,

    inventory_viewers: HashMap<InventoryId, HashSet<ClientId>>,
}

struct VisionClient {
    view: Region<V2>,
    visible_entities: RefcountedMap<EntityId, ()>,
    visible_inventories: RefcountedMap<InventoryId, ()>,
    visible_structures: RefcountedMap<StructureId, ()>,
}

struct VisionEntity {
    area: SmallSet<V2>,
    viewers: HashSet<ClientId>,
}

struct VisionStructure {
    area: SmallSet<V2>,
    viewers: HashSet<ClientId>,
}

#[allow(unused_variables)]
pub trait Hooks {
    fn on_chunk_appear(&mut self, cid: ClientId, pos: V2) {}
    fn on_chunk_disappear(&mut self, cid: ClientId, pos: V2) {}
    fn on_chunk_update(&mut self, cid: ClientId, pos: V2) {}

    fn on_entity_appear(&mut self, cid: ClientId, eid: EntityId) {}
    fn on_entity_disappear(&mut self, cid: ClientId, eid: EntityId) {}
    fn on_entity_motion_update(&mut self, cid: ClientId, eid: EntityId) {}
    fn on_entity_appearance_update(&mut self, cid: ClientId, eid: EntityId) {}

    fn on_structure_appear(&mut self, cid: ClientId, sid: StructureId) {}
    fn on_structure_disappear(&mut self, cid: ClientId, sid: StructureId) {}

    fn on_inventory_appear(&mut self, cid: ClientId, iid: InventoryId) {}
    fn on_inventory_disappear(&mut self, cid: ClientId, iid: InventoryId) {}
    fn on_inventory_update(&mut self,
                           cid: ClientId,
                           iid: InventoryId,
                           item_id: ItemId,
                           old_count: u8,
                           new_count: u8) {}
}

pub struct NoHooks;
impl Hooks for NoHooks { }

impl Vision {
    pub fn new() -> Vision {
        Vision {
            clients: VecMap::new(),
            entities: VecMap::new(),
            structures: VecMap::new(),
            clients_by_chunk: HashMap::new(),
            entities_by_chunk: HashMap::new(),
            structures_by_chunk: HashMap::new(),
            loaded_chunks: HashSet::new(),
            inventory_viewers: HashMap::new(),
        }
    }
}

impl Vision {
    pub fn add_client<H>(&mut self,
                         cid: ClientId,
                         view: Region<V2>,
                         h: &mut H)
            where H: Hooks {
        debug!("{:?} created", cid);
        self.clients.insert(cid.unwrap() as usize, VisionClient::new());
        self.set_client_view(cid, view, h);
    }

    pub fn remove_client<H>(&mut self,
                            cid: ClientId,
                            h: &mut H)
            where H: Hooks {
        debug!("{:?} destroyed", cid);
        self.set_client_view(cid, Region::empty(), h);
        self.clients.remove(&(cid.unwrap() as usize));
    }

    pub fn set_client_view<H>(&mut self,
                              cid: ClientId,
                              new_view: Region<V2>,
                              h: &mut H)
            where H: Hooks {
        let raw_cid = cid.unwrap() as usize;
        let client = unwrap_or!(self.clients.get_mut(&raw_cid));
        let old_view = mem::replace(&mut client.view, new_view);
        let entities = &mut self.entities;
        let structures = &mut self.structures;

        for p in new_view.points().filter(|&p| !old_view.contains(p)) {
            for &eid in self.entities_by_chunk.get(&p).map(|x| x.iter()).unwrap_iter() {
                client.visible_entities.retain(eid, || {
                    debug!("{:?} moved: ++{:?}", cid, eid);
                    h.on_entity_appear(cid, eid);
                    h.on_entity_motion_update(cid, eid);
                    entities[eid.unwrap() as usize].viewers.insert(cid);
                });
            }

            for &sid in self.structures_by_chunk.get(&p).map(|x| x.iter()).unwrap_iter() {
                client.visible_structures.retain(sid, || {
                    debug!("{:?} moved: ++{:?}", cid, sid);
                    h.on_structure_appear(cid, sid);
                    structures[sid.unwrap() as usize].viewers.insert(cid);
                });
            }

            if self.loaded_chunks.contains(&p) {
                debug!("{:?} moved: ++chunk {:?}", cid, p);
                h.on_chunk_appear(cid, p);
                h.on_chunk_update(cid, p);
            }
            multimap_insert(&mut self.clients_by_chunk, p, cid);
        }

        for p in old_view.points().filter(|&p| !new_view.contains(p)) {
            for &eid in self.entities_by_chunk.get(&p).map(|x| x.iter()).unwrap_iter() {
                client.visible_entities.release(eid, |()| {
                    debug!("{:?} moved: --{:?}", cid, eid);
                    h.on_entity_disappear(cid, eid);
                    entities[eid.unwrap() as usize].viewers.remove(&cid);
                });
            }

            for &sid in self.structures_by_chunk.get(&p).map(|x| x.iter()).unwrap_iter() {
                client.visible_structures.release(sid, |()| {
                    debug!("{:?} moved: --{:?}", cid, sid);
                    h.on_structure_disappear(cid, sid);
                    structures[sid.unwrap() as usize].viewers.remove(&cid);
                });
            }

            if self.loaded_chunks.contains(&p) {
                debug!("{:?} moved: --{:?}", cid, p);
                h.on_chunk_disappear(cid, p);
            }

            multimap_remove(&mut self.clients_by_chunk, p, cid);
        }
    }

    pub fn client_view_area(&self, cid: ClientId) -> Option<Region<V2>> {
        self.clients.get(&(cid.unwrap() as usize)).map(|c| c.view)
    }


    pub fn add_entity<H>(&mut self,
                         eid: EntityId,
                         area: SmallSet<V2>,
                         h: &mut H)
            where H: Hooks {
        self.entities.insert(eid.unwrap() as usize, VisionEntity::new());
        self.set_entity_area(eid, area, h);
    }

    pub fn remove_entity<H>(&mut self,
                            eid: EntityId,
                            h: &mut H)
            where H: Hooks {
        self.set_entity_area(eid, SmallSet::new(), h);
        self.entities.remove(&(eid.unwrap() as usize));
    }

    pub fn set_entity_area<H>(&mut self,
                              eid: EntityId,
                              new_area: SmallSet<V2>,
                              h: &mut H)
            where H: Hooks {
        let raw_eid = eid.unwrap() as usize;
        let entity = &mut self.entities[raw_eid];

        let old_area = mem::replace(&mut entity.area, SmallSet::new());

        for &p in new_area.iter().filter(|&p| !old_area.contains(p)) {
            for &cid in self.clients_by_chunk.get(&p).map(|x| x.iter()).unwrap_iter() {
                self.clients[cid.unwrap() as usize].visible_entities.retain(eid, || {
                    debug!("{:?} moved: ++{:?}", eid, cid);
                    h.on_entity_appear(cid, eid);
                    entity.viewers.insert(cid);
                });
            }
            multimap_insert(&mut self.entities_by_chunk, p, eid);
        }

        for &p in old_area.iter().filter(|&p| !new_area.contains(p)) {
            for &cid in self.clients_by_chunk.get(&p).map(|x| x.iter()).unwrap_iter() {
                self.clients[cid.unwrap() as usize].visible_entities.release(eid, |()| {
                    debug!("{:?} moved: --{:?}", eid, cid);
                    h.on_entity_disappear(cid, eid);
                    entity.viewers.remove(&cid);
                });
            }
            multimap_remove(&mut self.entities_by_chunk, p, eid);
        }

        for &cid in entity.viewers.iter() {
            debug!("{:?} moved: **{:?}", eid, cid);
            h.on_entity_motion_update(cid, eid);
        }

        entity.area = new_area;
    }

    pub fn update_entity_appearance<H>(&mut self,
                                       eid: EntityId,
                                       h: &mut H)
            where H: Hooks {
        let raw_eid = eid.unwrap() as usize;
        let entity = &self.entities[raw_eid];

        for &cid in entity.viewers.iter() {
            h.on_entity_appearance_update(cid, eid);
        }
    }


    pub fn add_structure<H>(&mut self,
                            sid: StructureId,
                            area: SmallSet<V2>,
                            h: &mut H)
            where H: Hooks {
        self.structures.insert(sid.unwrap() as usize, VisionStructure::new());
        let structure = &mut self.structures[sid.unwrap() as usize];

        for &p in area.iter() {
            for &cid in self.clients_by_chunk.get(&p).map(|x| x.iter()).unwrap_iter() {
                self.clients[cid.unwrap() as usize].visible_structures.retain(sid, || {
                    debug!("{:?} moved: ++{:?}", sid, cid);
                    h.on_structure_appear(cid, sid);
                    structure.viewers.insert(cid);
                });
            }
            multimap_insert(&mut self.structures_by_chunk, p, sid);
        }

        structure.area = area;
    }

    pub fn remove_structure<H>(&mut self,
                               sid: StructureId,
                               h: &mut H)
            where H: Hooks {
        let structure = self.structures.remove(&(sid.unwrap() as usize)).unwrap();
        for &p in structure.area.iter() {
            for &cid in self.clients_by_chunk.get(&p).map(|x| x.iter()).unwrap_iter() {
                self.clients[cid.unwrap() as usize].visible_structures.release(sid, |()| {
                    debug!("{:?} moved: --{:?}", sid, cid);
                    h.on_structure_disappear(cid, sid);
                });
            }
            multimap_remove(&mut self.structures_by_chunk, p, sid);
        }
    }


    pub fn add_chunk<H>(&mut self,
                        pos: V2,
                        h: &mut H)
            where H: Hooks {
        self.loaded_chunks.insert(pos);
        for &cid in self.clients_by_chunk.get(&pos).map(|x| x.iter()).unwrap_iter() {
            debug!("chunk {:?} created: ++{:?}", pos, cid);
            h.on_chunk_appear(cid, pos);
            h.on_chunk_update(cid, pos);
        }
    }

    pub fn remove_chunk<H>(&mut self,
                           pos: V2,
                           h: &mut H)
            where H: Hooks {
        for &cid in self.clients_by_chunk.get(&pos).map(|x| x.iter()).unwrap_iter() {
            debug!("chunk {:?} destroyed: --{:?}", pos, cid);
            h.on_chunk_disappear(cid, pos);
        }
        self.loaded_chunks.remove(&pos);
    }

    pub fn update_chunk<H>(&mut self,
                           pos: V2,
                           h: &mut H)
            where H: Hooks {
        for &cid in self.clients_by_chunk.get(&pos).map(|x| x.iter()).unwrap_iter() {
            debug!("chunk {:?} updated: **{:?}", pos, cid);
            h.on_chunk_update(cid, pos);
        }
    }


    pub fn subscribe_inventory<H>(&mut self,
                                  cid: ClientId,
                                  iid: InventoryId,
                                  h: &mut H)
            where H: Hooks {
        let client = unwrap_or!(self.clients.get_mut(&(cid.unwrap() as usize)));
        let inventory_viewers = &mut self.inventory_viewers;

        client.visible_inventories.retain(iid, || {
            multimap_insert(inventory_viewers, iid, cid);
            h.on_inventory_appear(cid, iid);
        });
    }

    pub fn unsubscribe_inventory<H>(&mut self,
                                    cid: ClientId,
                                    iid: InventoryId,
                                    h: &mut H)
            where H: Hooks {
        let client = unwrap_or!(self.clients.get_mut(&(cid.unwrap() as usize)));
        let inventory_viewers = &mut self.inventory_viewers;

        client.visible_inventories.release(iid, |()| {
            multimap_remove(inventory_viewers, iid, cid);
            h.on_inventory_disappear(cid, iid);
        });
    }

    pub fn update_inventory<H>(&mut self,
                               iid: InventoryId,
                               item_id: ItemId,
                               old_count: u8,
                               new_count: u8,
                               h: &mut H)
            where H: Hooks {
        let cids = unwrap_or!(self.inventory_viewers.get(&iid));
        for &cid in cids.iter() {
            h.on_inventory_update(cid, iid, item_id, old_count, new_count);
        }
    }
}

impl VisionClient {
    fn new() -> VisionClient {
        VisionClient {
            view: Region::empty(),
            visible_entities: RefcountedMap::new(),
            visible_inventories: RefcountedMap::new(),
            visible_structures: RefcountedMap::new(),
        }
    }
}

impl VisionEntity {
    fn new() -> VisionEntity {
        VisionEntity {
            area: SmallSet::new(),
            viewers: HashSet::new(),
        }
    }
}

impl VisionStructure {
    fn new() -> VisionStructure {
        VisionStructure {
            area: SmallSet::new(),
            viewers: HashSet::new(),
        }
    }
}


macro_rules! gen_Fragment {
    ($( fn $name:ident($($arg:ident: $arg_ty:ty),*); )*) => {
        pub trait Fragment<'d> {
            type H: Hooks;
            fn with_hooks<F, R>(&mut self, f: F) -> R
                where F: FnOnce(&mut Vision, &mut Self::H) -> R;

            $(
                fn $name(&mut self, $($arg: $arg_ty),*) {
                    self.with_hooks(|sys, hooks| {
                        sys.$name($($arg,)* hooks)
                    })
                }
            )*
        }
    };
}

gen_Fragment! {
    fn add_client(cid: ClientId, view: Region<V2>);
    fn remove_client(cid: ClientId);
    fn set_client_view(cid: ClientId, view: Region<V2>);

    fn add_entity(eid: EntityId, area: SmallSet<V2>);
    fn remove_entity(eid: EntityId);
    fn set_entity_area(eid: EntityId, area: SmallSet<V2>);
    fn update_entity_appearance(eid: EntityId);

    fn add_structure(sid: StructureId, area: SmallSet<V2>);
    fn remove_structure(sid: StructureId);

    fn add_chunk(pos: V2);
    fn remove_chunk(pos: V2);
    fn update_chunk(pos: V2);

    fn subscribe_inventory(cid: ClientId, iid: InventoryId);
    fn unsubscribe_inventory(cid: ClientId, iid: InventoryId);
    fn update_inventory(iid: InventoryId,
                        item_id: ItemId,
                        old_count: u8,
                        new_count: u8);
}

