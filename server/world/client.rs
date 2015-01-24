use std::collections::HashSet;

use physics::v3::scalar;

use input::InputBits;
use types::Time;
use types::{StableId, ClientId, EntityId, InventoryId};
use util::NO_STABLE_ID;
use view::ViewState;
use world::{World, Update};
use world::object::{Object, ObjectRef, ObjectRefT};
use world::entity;
use world::entity::EntityRef;


pub struct Client {
    entity: Option<EntityId>,
    current_input: InputBits,
    chunk_offset: (u8, u8),
    view_state: ViewState,

    stable_id: StableId,
    child_entities: HashSet<EntityId>,
    child_inventories: HashSet<InventoryId>,
}
impl_IntrusiveStableId!(Client, stable_id);

impl Client {
    pub fn new(chunk_offset: (u8, u8)) -> Client {
        Client {
            entity: None,
            current_input: InputBits::empty(),
            chunk_offset: chunk_offset,
            view_state: ViewState::new(scalar(0)),

            stable_id: NO_STABLE_ID,
            child_entities: HashSet::new(),
            child_inventories: HashSet::new(),
        }
    }

    pub fn check_invariant(&self, id: ClientId, world: &World) -> bool {
        use util::Stable;

        let mut ok = true;

        if let Some(eid) = self.entity {
            if let Some(e) = world.entities.get(eid) {
                let attach = e.attachment();
                check!(ok, attach == entity::Attach::Client(id),
                       "client {} pawn entity {} is wrongly attached to {:?}",
                       id, eid, attach);
            } else {
                bad!(ok, "client {} pawn entity {} does not exist", id, eid);
            }
        }

        if self.stable_id != NO_STABLE_ID {
            let opt_lookup_id = world.clients.get_id(Stable(self.stable_id));
            if let Some(lookup_id) = opt_lookup_id {
                check!(ok, lookup_id == id,
                       "client {} stable id {} actually belongs to client {}",
                       id, self.stable_id, lookup_id);
            } else {
                bad!(ok, "client {} stable id {} is not valid",
                     id, self.stable_id);
            }
        }

        for &eid in self.child_entities.iter() {
            if let Some(e) = world.entities.get(eid) {
                let attach = e.attachment();
                check!(ok, attach == entity::Attach::Client(id),
                       "client {} child entity {} is wrongly attached to {:?}",
                       id, eid, attach);
            } else {
                bad!(ok, "client {} child entity {} does not exist", id, eid);
            }
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
}

pub trait ClientInternal {
    fn child_entities(&self) -> &HashSet<EntityId>;
    fn child_entities_mut(&mut self) -> &mut HashSet<EntityId>;
}

impl ClientInternal for Client {
    fn child_entities(&self) -> &HashSet<EntityId> {
        &self.child_entities
    }

    fn child_entities_mut(&mut self) -> &mut HashSet<EntityId> {
        &mut self.child_entities
    }
}

pub trait ClientRef<'d>: ObjectRefT<'d, Client> {
    fn set_entity(&mut self, now: Time, eid: EntityId) -> Option<EntityId> {
        let cid = self.id();

        let pos;
        {
            let mut entity = self.world_mut().entity_mut(eid);
            entity.set_attachment(entity::Attach::Client(cid));
            pos = entity.pos(now);
        }

        let old_eid;
        {
            let obj = self.obj_mut();
            old_eid = obj.entity;
            obj.entity = Some(eid);
            obj.view_state = ViewState::new(pos);
        }

        self.world_mut().record(Update::ClientViewReset(cid));
        old_eid
    }
}

impl<'a, 'd> ClientRef<'d> for ObjectRef<'a, 'd, Client> { }
