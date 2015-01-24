use std::collections::{HashMap, HashSet};
use std::mem::replace;

use physics::CHUNK_SIZE;
use physics::v3::{Vn, V3, scalar};

use types::Time;
use types::{StableId, ClientId, EntityId, InventoryId};
use types::AnimId;
use util::NO_STABLE_ID;
use world::{World, Update};
use world::object::{Object, ObjectRef, ObjectRefT};
use world::client::ClientInternal;


#[derive(Copy, PartialEq, Eq, Show)]
pub enum Attach {
    World,
    Chunk,
    Client(ClientId),
}

pub struct Motion;

pub struct Entity {
    motion: Motion,
    anim: AnimId,
    facing: V3,

    stable_id: StableId,
    attachment: Attach,
    child_inventories: HashSet<InventoryId>,
}
impl_IntrusiveStableId!(Entity, stable_id);

impl Entity {
    pub fn new(pos: V3, anim: AnimId) -> Entity {
        Entity {
            motion: Motion,
            anim: anim,
            facing: V3::new(1, 0, 0),

            stable_id: NO_STABLE_ID,
            attachment: Attach::World,
            child_inventories: HashSet::new(),
        }
    }

    pub fn check_invariant(&self, now: Time, id: EntityId, world: &World) -> bool {
        use util::Stable;

        let mut ok = true;

        if self.stable_id != NO_STABLE_ID {
            let opt_lookup_id = world.entities.get_id(Stable(self.stable_id));
            if let Some(lookup_id) = opt_lookup_id {
                check!(ok, lookup_id == id,
                       "entity {} stable id {} actually belongs to entity {}",
                       id, self.stable_id, lookup_id);
            } else {
                bad!(ok, "entity {} stable id {} is not valid",
                     id, self.stable_id);
            }
        }

        match self.attachment {
            Attach::World => {},
            Attach::Chunk => {
                // TODO: check that Motion does not involve any actual movement
                let chunk_id = self.pos(0).reduce().div_floor(scalar(CHUNK_SIZE));
                check!(ok, world.terrain_chunks.get(&chunk_id).is_some(),
                       "entity {} parent chunk {:?} does not exist",
                       id, chunk_id);
            },
            Attach::Client(cid) => {
                if let Some(client) = world.clients.get(cid) {
                    check!(ok, client.child_entities().contains(&id),
                           "entity {} parent client {} does not list entity as child",
                           id, cid);
                } else {
                    bad!(ok, "entity {} parent client {} does not exist", id, cid);
                }
            },
        }

        /*
        for &iid in self.child_inventories.iter() {
            if let Some(i) = world.inventories.get(iid) {
                let attach = i.attachment();
                check!(ok, attach == inventory::Attach::Client(id),
                       "client {} child inventory {} is wrongly attached to {:?}",
                       id, iid, attach);
            } else {
                bad!(ok, "client {} child inventory {} does not exist", id, iid);
            }
        }
        */

        ok
    }

    pub fn pos(&self, now: Time) -> V3 {
        // TODO
        V3::new(1, 2, 3)
    }

    pub fn attachment(&self) -> Attach {
        self.attachment
    }
}

pub trait EntityRef<'d>: ObjectRefT<'d, Entity> {
    fn set_attachment(&mut self, attachment: Attach) {
        // TODO: if `attachment` is `Chunk`, check that `self.motion` is stationary
        let old_attach = replace(&mut self.obj_mut().attachment, attachment);

        if old_attach == attachment {
            return;
        }

        let eid = self.id();

        if let Attach::Client(cid) = old_attach {
            self.world_mut().client_mut(cid).child_entities_mut().remove(&eid);
        }

        if let Attach::Client(cid) = attachment {
            self.world_mut().client_mut(cid).child_entities_mut().insert(eid);
        } else if attachment == Attach::Chunk {
            let chunk_id = self.pos(0).reduce().div_floor(scalar(CHUNK_SIZE));
            assert!(self.world().terrain_chunks.get(&chunk_id).is_some(),
                    "can't attach entity {} to Chunk because chunk {:?} is not loaded",
                    eid, chunk_id);
        }
    }
}

impl<'a, 'd> EntityRef<'d> for ObjectRef<'a, 'd, Entity> { }
