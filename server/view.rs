use std::collections::{HashMap, HashSet, VecMap};
use std::mem;

use physics::v3::{Vn, V3, V2, scalar, Region};
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
}

struct VisionClient {
    view: Region<V2>,
    visible_entities: RefcountedMap<EntityId, ()>,
}

struct VisionEntity {
    area: SmallSet<V2>,
    viewers: HashSet<ClientId>,
}

#[allow(unused_variables)]
pub trait VisionCallbacks {
    fn chunk_appear(&mut self, cid: ClientId, pos: V2) {}
    fn chunk_disappear(&mut self, cid: ClientId, pos: V2) {}
    fn chunk_update(&mut self, cid: ClientId, pos: V2) {}

    fn entity_appear(&mut self, cid: ClientId, eid: EntityId) {}
    fn entity_disappear(&mut self, cid: ClientId, eid: EntityId) {}
    fn entity_update(&mut self, cid: ClientId, eid: EntityId) {}
}

impl Vision {
    pub fn new() -> Vision {
        Vision {
            clients: VecMap::new(),
            entities: VecMap::new(),
            clients_by_chunk: HashMap::new(),
            entities_by_chunk: HashMap::new(),
            loaded_chunks: HashSet::new(),
        }
    }
}

impl Vision {
    pub fn add_client<CB>(&mut self,
                          cid: ClientId,
                          view: Region<V2>,
                          cb: &mut CB)
            where CB: VisionCallbacks {
        self.clients.insert(cid.unwrap() as usize, VisionClient::new());
        self.set_client_view(cid, view, cb);
    }

    pub fn remove_client<CB>(&mut self,
                             cid: ClientId,
                             cb: &mut CB)
            where CB: VisionCallbacks {
        self.set_client_view(cid, Region::empty(), cb);
        self.clients.remove(&(cid.unwrap() as usize));
    }

    pub fn set_client_view<CB>(&mut self,
                               cid: ClientId,
                               new_view: Region<V2>,
                               cb: &mut CB)
            where CB: VisionCallbacks {
        let raw_cid = cid.unwrap() as usize;
        let client = &mut self.clients[raw_cid];
        let old_view = mem::replace(&mut client.view, new_view);
        let entities = &mut self.entities;

        for p in old_view.points().filter(|&p| !new_view.contains(p)) {
            for &eid in self.entities_by_chunk.get(&p).map(|x| x.iter()).unwrap_iter() {
                client.visible_entities.release(eid, |()| {
                    cb.entity_disappear(cid, eid);
                    entities[eid.unwrap() as usize].viewers.remove(&cid);
                });
            }

            if self.loaded_chunks.contains(&p) {
                cb.chunk_disappear(cid, p);
            }

            multimap_remove(&mut self.clients_by_chunk, p, cid);
        }

        for p in new_view.points().filter(|&p| !old_view.contains(p)) {
            for &eid in self.entities_by_chunk.get(&p).map(|x| x.iter()).unwrap_iter() {
                client.visible_entities.retain(eid, || {
                    cb.entity_appear(cid, eid);
                    cb.entity_update(cid, eid);
                    entities[eid.unwrap() as usize].viewers.insert(cid);
                });
            }

            if self.loaded_chunks.contains(&p) {
                cb.chunk_appear(cid, p);
                cb.chunk_update(cid, p);
            }
            multimap_insert(&mut self.clients_by_chunk, p, cid);
        }
    }

    pub fn client_view_area(&self, cid: ClientId) -> Option<Region<V2>> {
        self.clients.get(&(cid.unwrap() as usize)).map(|c| c.view)
    }


    pub fn add_entity<CB>(&mut self,
                          eid: EntityId,
                          area: SmallSet<V2>,
                          cb: &mut CB)
            where CB: VisionCallbacks {
        self.entities.insert(eid.unwrap() as usize, VisionEntity::new());
        self.set_entity_area(eid, area, cb);
    }

    pub fn remove_entity<CB>(&mut self,
                             eid: EntityId,
                             cb: &mut CB)
            where CB: VisionCallbacks {
        self.set_entity_area(eid, SmallSet::new(), cb);
        self.entities.remove(&(eid.unwrap() as usize));
    }

    pub fn set_entity_area<CB>(&mut self,
                               eid: EntityId,
                               new_area: SmallSet<V2>,
                               cb: &mut CB)
            where CB: VisionCallbacks {
        let raw_eid = eid.unwrap() as usize;
        let entity = &mut self.entities[raw_eid];

        let old_area = mem::replace(&mut entity.area, SmallSet::new());

        for &p in old_area.iter().filter(|&p| !new_area.contains(p)) {
            for &cid in self.clients_by_chunk.get(&p).map(|x| x.iter()).unwrap_iter() {
                self.clients[cid.unwrap() as usize].visible_entities.release(eid, |()| {
                    cb.entity_disappear(cid, eid);
                    entity.viewers.remove(&cid);
                });
            }
            multimap_remove(&mut self.entities_by_chunk, p, eid);
        }

        for &p in new_area.iter().filter(|&p| !old_area.contains(p)) {
            for &cid in self.clients_by_chunk.get(&p).map(|x| x.iter()).unwrap_iter() {
                self.clients[cid.unwrap() as usize].visible_entities.retain(eid, || {
                    cb.entity_appear(cid, eid);
                    entity.viewers.insert(cid);
                });
            }
            multimap_insert(&mut self.entities_by_chunk, p, eid);
        }

        for &cid in entity.viewers.iter() {
            cb.entity_update(cid, eid);
        }

        entity.area = new_area;
    }


    pub fn add_chunk<CB>(&mut self,
                         pos: V2,
                         cb: &mut CB)
            where CB: VisionCallbacks {
        self.loaded_chunks.insert(pos);
        for &cid in self.clients_by_chunk.get(&pos).map(|x| x.iter()).unwrap_iter() {
            cb.chunk_appear(cid, pos);
            cb.chunk_update(cid, pos);
        }
    }

    pub fn remove_chunk<CB>(&mut self,
                            pos: V2,
                            cb: &mut CB)
            where CB: VisionCallbacks {
        for &cid in self.clients_by_chunk.get(&pos).map(|x| x.iter()).unwrap_iter() {
            cb.chunk_disappear(cid, pos);
        }
        self.loaded_chunks.remove(&pos);
    }

    pub fn update_chunk<CB>(&mut self,
                            pos: V2,
                            cb: &mut CB)
            where CB: VisionCallbacks {
        for &cid in self.clients_by_chunk.get(&pos).map(|x| x.iter()).unwrap_iter() {
            cb.chunk_update(cid, pos);
        }
    }
}

impl VisionClient {
    fn new() -> VisionClient {
        VisionClient {
            view: Region::empty(),
            visible_entities: RefcountedMap::new(),
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
