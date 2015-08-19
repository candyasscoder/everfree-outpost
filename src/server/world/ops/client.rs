use std::borrow::ToOwned;
use std::collections::HashSet;
use std::mem::replace;

use types::*;
use util::stable_id_map::NO_STABLE_ID;

use input::InputBits;
use world::EntityAttachment;

use world::Client;
use world::{Fragment, Hooks};
use world::ops::{self, OpResult};


pub fn create<'d, F>(f: &mut F,
                     name: &str) -> OpResult<ClientId>
        where F: Fragment<'d> {
    let c = Client {
        name: name.to_owned(),
        pawn: None,
        current_input: InputBits::empty(),

        stable_id: NO_STABLE_ID,
        child_entities: HashSet::new(),
        child_inventories: HashSet::new(),
    };

    let cid = unwrap!(f.world_mut().clients.insert(c));
    f.with_hooks(|h| h.on_client_create(cid));
    Ok(cid)
}

pub fn create_unchecked<'d, F>(f: &mut F) -> ClientId
        where F: Fragment<'d> {
    let cid = f.world_mut().clients.insert(Client {
        name: String::new(),
        pawn: None,
        current_input: InputBits::empty(),

        stable_id: NO_STABLE_ID,
        child_entities: HashSet::new(),
        child_inventories: HashSet::new(),
    }).unwrap();     // Shouldn't fail when stable_id == NO_STABLE_ID
    cid
}

pub fn destroy<'d, F>(f: &mut F,
                      cid: ClientId) -> OpResult<()>
        where F: Fragment<'d> {
    let c = unwrap!(f.world_mut().clients.remove(cid));
    // Further lookup failures indicate an invariant violation.

    for &eid in c.child_entities.iter() {
        // TODO: do we really want .unwrap() here?
        ops::entity::destroy(f, eid).unwrap();
    }

    for &iid in c.child_inventories.iter() {
        ops::inventory::destroy(f, iid).unwrap();
    }

    f.with_hooks(|h| h.on_client_destroy(cid));
    Ok(())
}

pub fn set_pawn<'d, F>(f: &mut F,
                       cid: ClientId,
                       eid: EntityId) -> OpResult<Option<EntityId>>
        where F: Fragment<'d> {
    try!(ops::entity::attach(f, eid, EntityAttachment::Client(cid)));
    let old_eid;

    {
        let c = unwrap!(f.world_mut().clients.get_mut(cid));
        // We know 'eid' is valid because the 'entity_attach' above succeeded.
        old_eid = replace(&mut c.pawn, Some(eid));
    }

    f.with_hooks(|h| h.on_client_change_pawn(cid, old_eid, Some(eid)));
    Ok(old_eid)
}

pub fn clear_pawn<'d, F>(f: &mut F,
                         cid: ClientId) -> OpResult<Option<EntityId>>
        where F: Fragment<'d> {
    let old_eid;
    {
        let c = unwrap!(f.world_mut().clients.get_mut(cid));
        // NB: Keep this behavior in sync with entity_destroy.
        old_eid = replace(&mut c.pawn, None);
    }

    f.with_hooks(|h| h.on_client_change_pawn(cid, old_eid, None));
    Ok(old_eid)
}
