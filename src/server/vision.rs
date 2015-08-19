use std::collections::{HashMap, HashSet, VecMap};
use std::mem;

use libphysics::{CHUNK_SIZE, TILE_SIZE};

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


type ViewerId = ClientId;

pub struct Vision {
    viewers: VecMap<Viewer>,
    viewers_by_pos: HashMap<(PlaneId, V2), HashSet<ViewerId>>,

    entities: VecMap<Entity>,
    terrain_chunks: VecMap<TerrainChunk>,
    structures: VecMap<Structure>,

    // NB: PLANE_LIMBO gets special treatment so that it always appears empty, no matter what is
    // actually present.  This is done by skipping insertions into x_by_pos when the PlaneId is
    // PLANE_LIMBO.  Thus, none of the x_by_pos maps should ever have PLANE_LIMBO in theer keys.
    entities_by_pos: HashMap<(PlaneId, V2), HashSet<EntityId>>,
    terrain_chunks_by_pos: HashMap<(PlaneId, V2), HashSet<TerrainChunkId>>,
    structures_by_pos: HashMap<(PlaneId, V2), HashSet<StructureId>>,

    inventory_viewers: HashMap<InventoryId, HashSet<ViewerId>>,
}


struct Viewer {
    plane: PlaneId,
    view: Region<V2>,

    visible_entities: RefcountedMap<EntityId, ()>,
    visible_terrain_chunks: RefcountedMap<TerrainChunkId, ()>,
    visible_structures: RefcountedMap<StructureId, ()>,
    visible_inventories: RefcountedMap<InventoryId, ()>,
}

struct Entity {
    plane: PlaneId,
    area: SmallSet<V2>,
    viewers: HashSet<ViewerId>,
}

struct TerrainChunk {
    plane: PlaneId,
    cpos: V2,
    viewers: HashSet<ViewerId>,
}

struct Structure {
    plane: PlaneId,
    area: SmallSet<V2>,
    viewers: HashSet<ViewerId>,
}


#[allow(unused_variables)]
pub trait Hooks {
    fn on_entity_appear(&mut self, cid: ClientId, eid: EntityId) {}
    fn on_entity_disappear(&mut self, cid: ClientId, eid: EntityId) {}
    fn on_entity_motion_update(&mut self, cid: ClientId, eid: EntityId) {}
    fn on_entity_appearance_update(&mut self, cid: ClientId, eid: EntityId) {}

    fn on_plane_change(&mut self,
                       cid: ClientId,
                       old_pid: PlaneId,
                       new_pid: PlaneId) {}

    fn on_terrain_chunk_appear(&mut self,
                               cid: ClientId,
                               tcid: TerrainChunkId,
                               cpos: V2) {}
    fn on_terrain_chunk_disappear(&mut self,
                                  cid: ClientId,
                                  tcid: TerrainChunkId,
                                  cpos: V2) {}
    fn on_terrain_chunk_update(&mut self,
                               cid: ClientId,
                               tcid: TerrainChunkId,
                               cpos: V2) {}

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
            viewers: VecMap::new(),
            viewers_by_pos: HashMap::new(),

            entities: VecMap::new(),
            terrain_chunks: VecMap::new(),
            structures: VecMap::new(),

            entities_by_pos: HashMap::new(),
            terrain_chunks_by_pos: HashMap::new(),
            structures_by_pos: HashMap::new(),

            inventory_viewers: HashMap::new(),
        }
    }
}

impl Vision {
    pub fn add_client<H>(&mut self,
                         cid: ClientId,
                         plane: PlaneId,
                         view: Region<V2>,
                         h: &mut H)
            where H: Hooks {
        trace!("{:?} created", cid);
        self.viewers.insert(cid.unwrap() as usize, Viewer::new());
        self.set_client_view(cid, plane, view, h);
    }

    pub fn remove_client<H>(&mut self,
                            cid: ClientId,
                            h: &mut H)
            where H: Hooks {
        trace!("{:?} destroyed", cid);
        self.set_client_view(cid, PLANE_LIMBO, Region::empty(), h);
        self.viewers.remove(&(cid.unwrap() as usize));
    }

    // This code is carefully arranged to produce events in the proper order.  Specifically, when a
    // single update produces both "gone" and "appear" events, all "gone" events should appear
    // before all "appear" events.  This avoids giving an inconsistent view, in which (for example)
    // two structures more than `VIEW_SIZE` distance apart are visible at the same time.

    pub fn set_client_view<H>(&mut self,
                              cid: ClientId,
                              new_plane: PlaneId,
                              new_view: Region<V2>,
                              h: &mut H)
            where H: Hooks {
        let raw_cid = cid.unwrap() as usize;
        let viewer = unwrap_or!(self.viewers.get_mut(&raw_cid));
        let old_plane = mem::replace(&mut viewer.plane, new_plane);
        let old_view = mem::replace(&mut viewer.view, new_view);
        let plane_change = old_plane != new_plane;

        let entities = &mut self.entities;
        let terrain_chunks = &mut self.terrain_chunks;
        let structures = &mut self.structures;

        for p in old_view.points().filter(|&p| !new_view.contains(p) || plane_change) {
            let pos = (old_plane, p);

            for &eid in self.entities_by_pos.get(&pos).map(|x| x.iter()).unwrap_iter() {
                viewer.visible_entities.release(eid, |()| {
                    trace!("{:?} moved: --{:?}", cid, eid);
                    h.on_entity_disappear(cid, eid);
                    entities[eid.unwrap() as usize].viewers.remove(&cid);
                });
            }

            for &tcid in self.terrain_chunks_by_pos.get(&pos).map(|x| x.iter()).unwrap_iter() {
                viewer.visible_terrain_chunks.release(tcid, |()| {
                    trace!("{:?} moved: --{:?}", cid, tcid);
                    let cpos = terrain_chunks[tcid.unwrap() as usize].cpos;
                    h.on_terrain_chunk_disappear(cid, tcid, cpos);
                    terrain_chunks[tcid.unwrap() as usize].viewers.remove(&cid);
                });
            }

            for &sid in self.structures_by_pos.get(&pos).map(|x| x.iter()).unwrap_iter() {
                viewer.visible_structures.release(sid, |()| {
                    trace!("{:?} moved: --{:?}", cid, sid);
                    h.on_structure_disappear(cid, sid);
                    structures[sid.unwrap() as usize].viewers.remove(&cid);
                });
            }

            multimap_remove(&mut self.viewers_by_pos, pos, cid);
        }

        if plane_change {
            h.on_plane_change(cid, old_plane, new_plane);
        }

        for p in new_view.points().filter(|&p| !old_view.contains(p) || plane_change) {
            let pos = (new_plane, p);

            for &eid in self.entities_by_pos.get(&pos).map(|x| x.iter()).unwrap_iter() {
                viewer.visible_entities.retain(eid, || {
                    trace!("{:?} moved: ++{:?}", cid, eid);
                    h.on_entity_appear(cid, eid);
                    entities[eid.unwrap() as usize].viewers.insert(cid);
                });
            }

            for &tcid in self.terrain_chunks_by_pos.get(&pos).map(|x| x.iter()).unwrap_iter() {
                viewer.visible_terrain_chunks.retain(tcid, || {
                    trace!("{:?} moved: ++{:?}", cid, tcid);
                    let cpos = terrain_chunks[tcid.unwrap() as usize].cpos;
                    h.on_terrain_chunk_appear(cid, tcid, cpos);
                    terrain_chunks[tcid.unwrap() as usize].viewers.insert(cid);
                });
            }

            for &sid in self.structures_by_pos.get(&pos).map(|x| x.iter()).unwrap_iter() {
                viewer.visible_structures.retain(sid, || {
                    trace!("{:?} moved: ++{:?}", cid, sid);
                    h.on_structure_appear(cid, sid);
                    structures[sid.unwrap() as usize].viewers.insert(cid);
                });
            }

            if new_plane != PLANE_LIMBO {
                multimap_insert(&mut self.viewers_by_pos, pos, cid);
            }
        }
    }

    pub fn client_view_plane(&self, cid: ClientId) -> Option<PlaneId> {
        self.viewers.get(&(cid.unwrap() as usize)).map(|c| c.plane)
    }

    pub fn client_view_area(&self, cid: ClientId) -> Option<Region<V2>> {
        self.viewers.get(&(cid.unwrap() as usize)).map(|c| c.view)
    }


    pub fn add_entity<H>(&mut self,
                         eid: EntityId,
                         plane: PlaneId,
                         area: SmallSet<V2>,
                         h: &mut H)
            where H: Hooks {
        trace!("{:?} created", eid);
        self.entities.insert(eid.unwrap() as usize, Entity::new());
        self.set_entity_area(eid, plane, area, h);
    }

    pub fn remove_entity<H>(&mut self,
                            eid: EntityId,
                            h: &mut H)
            where H: Hooks {
        trace!("{:?} destroyed", eid);
        self.set_entity_area(eid, PLANE_LIMBO, SmallSet::new(), h);
        self.entities.remove(&(eid.unwrap() as usize));
    }

    pub fn set_entity_area<H>(&mut self,
                              eid: EntityId,
                              new_plane: PlaneId,
                              new_area: SmallSet<V2>,
                              h: &mut H)
            where H: Hooks {
        let raw_eid = eid.unwrap() as usize;
        let entity = &mut self.entities[raw_eid];

        let old_plane = mem::replace(&mut entity.plane, new_plane);
        // SmallSet is non-Copy, so insert a dummy value here and set the real one later.
        let old_area = mem::replace(&mut entity.area, SmallSet::new());
        let plane_change = new_plane != old_plane;

        // This looks like a violation of "send `gone` before `appear`", but it's not.  There are
        // four cases:
        //  - Neither old nor new position is visible: Refcount remains unchanged (at zero).
        //  - Only old position is visible: First loop has no effect, second decrements refcount
        //    (possibly generating `gone` event).
        //  - Only new position is visible: First loop increments refcount (possibly generating
        //    `appeear` event), second has no effect.
        //  - Both old and new are visible: Since old position is visible, refcount is positive,
        //    First loop increments, and second decrements.  No events are generated because the
        //    refoucnt is positive the whole way through.
        for &p in new_area.iter().filter(|&p| !old_area.contains(p) || plane_change) {
            let pos = (new_plane, p);
            for &cid in self.viewers_by_pos.get(&pos).map(|x| x.iter()).unwrap_iter() {
                self.viewers[cid.unwrap() as usize].visible_entities.retain(eid, || {
                    trace!("{:?} moved: ++{:?}", eid, cid);
                    h.on_entity_appear(cid, eid);
                    entity.viewers.insert(cid);
                });
            }
            if new_plane != PLANE_LIMBO {
                multimap_insert(&mut self.entities_by_pos, pos, eid);
            }
        }

        for &p in old_area.iter().filter(|&p| !new_area.contains(p) || plane_change) {
            let pos = (old_plane, p);
            for &cid in self.viewers_by_pos.get(&pos).map(|x| x.iter()).unwrap_iter() {
                self.viewers[cid.unwrap() as usize].visible_entities.release(eid, |()| {
                    trace!("{:?} moved: --{:?}", eid, cid);
                    h.on_entity_disappear(cid, eid);
                    entity.viewers.remove(&cid);
                });
            }
            multimap_remove(&mut self.entities_by_pos, pos, eid);
        }

        for &cid in entity.viewers.iter() {
            trace!("{:?} moved: **{:?}", eid, cid);
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


    pub fn add_terrain_chunk<H>(&mut self,
                                tcid: TerrainChunkId,
                                plane: PlaneId,
                                cpos: V2,
                                h: &mut H)
            where H: Hooks {
        trace!("{:?} created @ {:?}, {:?}", tcid, plane, cpos);
        self.terrain_chunks.insert(tcid.unwrap() as usize, TerrainChunk::new());
        let terrain_chunk = &mut self.terrain_chunks[tcid.unwrap() as usize];

        // Structures don't move, so we can inline the two halves of the set_x_area logic.

        let pos = (plane, cpos);
        for &cid in self.viewers_by_pos.get(&pos).map(|x| x.iter()).unwrap_iter() {
            self.viewers[cid.unwrap() as usize].visible_terrain_chunks.retain(tcid, || {
                trace!("{:?} moved: ++{:?}", tcid, cid);
                h.on_terrain_chunk_appear(cid, tcid, cpos);
                terrain_chunk.viewers.insert(cid);
            });
        }
        if plane != PLANE_LIMBO {
            multimap_insert(&mut self.terrain_chunks_by_pos, pos, tcid);
        }

        terrain_chunk.plane = plane;
        terrain_chunk.cpos = cpos;
    }

    pub fn remove_terrain_chunk<H>(&mut self,
                                   tcid: TerrainChunkId,
                                   h: &mut H)
            where H: Hooks {
        trace!("{:?} destroyed", tcid);
        let terrain_chunk = self.terrain_chunks.remove(&(tcid.unwrap() as usize)).unwrap();

        let pos = (terrain_chunk.plane, terrain_chunk.cpos);
        for &cid in self.viewers_by_pos.get(&pos).map(|x| x.iter()).unwrap_iter() {
            self.viewers[cid.unwrap() as usize].visible_terrain_chunks.release(tcid, |()| {
                debug!("{:?} moved: --{:?}", tcid, cid);
                h.on_terrain_chunk_disappear(cid, tcid, terrain_chunk.cpos);
            });
        }
        multimap_remove(&mut self.terrain_chunks_by_pos, pos, tcid);
    }

    pub fn update_terrain_chunk<H>(&mut self,
                                   tcid: TerrainChunkId,
                                   h: &mut H)
            where H: Hooks {

        let raw_tcid = tcid.unwrap() as usize;
        let terrain_chunk = &self.terrain_chunks[raw_tcid];

        for &cid in terrain_chunk.viewers.iter() {
            h.on_terrain_chunk_update(cid, tcid, terrain_chunk.cpos);
        }
    }


    pub fn add_structure<H>(&mut self,
                            sid: StructureId,
                            plane: PlaneId,
                            area: SmallSet<V2>,
                            h: &mut H)
            where H: Hooks {
        self.structures.insert(sid.unwrap() as usize, Structure::new());
        let structure = &mut self.structures[sid.unwrap() as usize];

        // Structures don't move, so we can inline the two halves of the set_x_area logic.

        for &p in area.iter() {
            let pos = (plane, p);
            for &cid in self.viewers_by_pos.get(&pos).map(|x| x.iter()).unwrap_iter() {
                self.viewers[cid.unwrap() as usize].visible_structures.retain(sid, || {
                    debug!("{:?} moved: ++{:?}", sid, cid);
                    h.on_structure_appear(cid, sid);
                    structure.viewers.insert(cid);
                });
            }
            if plane != PLANE_LIMBO {
                multimap_insert(&mut self.structures_by_pos, pos, sid);
            }
        }

        structure.plane = plane;
        structure.area = area;
    }

    pub fn remove_structure<H>(&mut self,
                               sid: StructureId,
                               h: &mut H)
            where H: Hooks {
        let structure = self.structures.remove(&(sid.unwrap() as usize)).unwrap();
        for &p in structure.area.iter() {
            let pos = (structure.plane, p);
            for &cid in self.viewers_by_pos.get(&pos).map(|x| x.iter()).unwrap_iter() {
                self.viewers[cid.unwrap() as usize].visible_structures.release(sid, |()| {
                    debug!("{:?} moved: --{:?}", sid, cid);
                    h.on_structure_disappear(cid, sid);
                });
            }
            multimap_remove(&mut self.structures_by_pos, pos, sid);
        }
    }

    // TODO: handle structure template changes


    pub fn subscribe_inventory<H>(&mut self,
                                  cid: ClientId,
                                  iid: InventoryId,
                                  h: &mut H)
            where H: Hooks {
        let viewer = unwrap_or!(self.viewers.get_mut(&(cid.unwrap() as usize)));
        let inventory_viewers = &mut self.inventory_viewers;

        viewer.visible_inventories.retain(iid, || {
            multimap_insert(inventory_viewers, iid, cid);
            h.on_inventory_appear(cid, iid);
        });
    }

    pub fn unsubscribe_inventory<H>(&mut self,
                                    cid: ClientId,
                                    iid: InventoryId,
                                    h: &mut H)
            where H: Hooks {
        let viewer = unwrap_or!(self.viewers.get_mut(&(cid.unwrap() as usize)));
        let inventory_viewers = &mut self.inventory_viewers;

        viewer.visible_inventories.release(iid, |()| {
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

impl Viewer {
    fn new() -> Viewer {
        Viewer {
            plane: PLANE_LIMBO,
            view: Region::empty(),
            visible_entities: RefcountedMap::new(),
            visible_terrain_chunks: RefcountedMap::new(),
            visible_structures: RefcountedMap::new(),
            visible_inventories: RefcountedMap::new(),
        }
    }
}

impl Entity {
    fn new() -> Entity {
        Entity {
            plane: PLANE_LIMBO,
            area: SmallSet::new(),
            viewers: HashSet::new(),
        }
    }
}

impl TerrainChunk {
    fn new() -> TerrainChunk {
        TerrainChunk {
            plane: PLANE_LIMBO,
            cpos: scalar(0),
            viewers: HashSet::new(),
        }
    }
}

impl Structure {
    fn new() -> Structure {
        Structure {
            plane: PLANE_LIMBO,
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
    fn add_client(cid: ClientId, plane: PlaneId, view: Region<V2>);
    fn remove_client(cid: ClientId);
    fn set_client_view(cid: ClientId, plane: PlaneId, view: Region<V2>);

    fn add_entity(eid: EntityId, plane: PlaneId, area: SmallSet<V2>);
    fn remove_entity(eid: EntityId);
    fn set_entity_area(eid: EntityId, plane: PlaneId, area: SmallSet<V2>);
    fn update_entity_appearance(eid: EntityId);

    fn add_terrain_chunk(tcid: TerrainChunkId, plane: PlaneId, cpos: V2);
    fn remove_terrain_chunk(tcid: TerrainChunkId);
    fn update_terrain_chunk(tcid: TerrainChunkId);

    fn add_structure(sid: StructureId, plane: PlaneId, area: SmallSet<V2>);
    fn remove_structure(sid: StructureId);

    fn subscribe_inventory(cid: ClientId, iid: InventoryId);
    fn unsubscribe_inventory(cid: ClientId, iid: InventoryId);
    fn update_inventory(iid: InventoryId,
                        item_id: ItemId,
                        old_count: u8,
                        new_count: u8);
}

