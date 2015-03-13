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

    clients_by_chunk: HashMap<V2, HashSet<ClientId>>,
    entities_by_chunk: HashMap<V2, HashSet<EntityId>>,

    loaded_chunks: HashSet<V2>,

    inventory_viewers: HashMap<InventoryId, HashSet<ClientId>>,
}

struct VisionClient {
    view: Region<V2>,
    visible_entities: RefcountedMap<EntityId, ()>,
    visible_inventories: RefcountedMap<InventoryId, ()>,
}

struct VisionEntity {
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
    fn on_entity_update(&mut self, cid: ClientId, eid: EntityId) {}

    fn on_inventory_appear(&mut self, cid: ClientId, iid: InventoryId) {}
    fn on_inventory_disappear(&mut self, cid: ClientId, iid: InventoryId) {}
    fn on_inventory_update(&mut self,
                           cid: ClientId,
                           iid: InventoryId,
                           item_id: ItemId,
                           old_count: u8,
                           new_count: u8) {}
}

impl Vision {
    pub fn new() -> Vision {
        Vision {
            clients: VecMap::new(),
            entities: VecMap::new(),
            clients_by_chunk: HashMap::new(),
            entities_by_chunk: HashMap::new(),
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

        for p in old_view.points().filter(|&p| !new_view.contains(p)) {
            for &eid in self.entities_by_chunk.get(&p).map(|x| x.iter()).unwrap_iter() {
                client.visible_entities.release(eid, |()| {
                    debug!("{:?} moved: --{:?}", cid, eid);
                    h.on_entity_disappear(cid, eid);
                    entities[eid.unwrap() as usize].viewers.remove(&cid);
                });
            }

            if self.loaded_chunks.contains(&p) {
                debug!("{:?} moved: --{:?}", cid, p);
                h.on_chunk_disappear(cid, p);
            }

            multimap_remove(&mut self.clients_by_chunk, p, cid);
        }

        for p in new_view.points().filter(|&p| !old_view.contains(p)) {
            for &eid in self.entities_by_chunk.get(&p).map(|x| x.iter()).unwrap_iter() {
                client.visible_entities.retain(eid, || {
                    debug!("{:?} moved: ++{:?}", cid, eid);
                    h.on_entity_appear(cid, eid);
                    h.on_entity_update(cid, eid);
                    entities[eid.unwrap() as usize].viewers.insert(cid);
                });
            }

            if self.loaded_chunks.contains(&p) {
                debug!("{:?} moved: ++chunk {:?}", cid, p);
                h.on_chunk_appear(cid, p);
                h.on_chunk_update(cid, p);
            }
            multimap_insert(&mut self.clients_by_chunk, p, cid);
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

        for &cid in entity.viewers.iter() {
            debug!("{:?} moved: **{:?}", eid, cid);
            h.on_entity_update(cid, eid);
        }

        entity.area = new_area;
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
