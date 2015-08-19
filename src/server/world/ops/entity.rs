use std::collections::HashSet;
use std::mem::replace;

use types::*;
use util::{multimap_insert, multimap_remove};

use world::{Entity, EntityAttachment, Motion};
use world::{Fragment, Hooks};
use world::ops::{self, OpResult};


pub fn create<'d, F>(f: &mut F,
                     stable_pid: Stable<PlaneId>,
                     pos: V3,
                     anim: AnimId,
                     appearance: u32) -> OpResult<EntityId>
        where F: Fragment<'d> {
    let e = Entity {
        stable_plane: stable_pid,
        // Initialization of `plane` is handled in `post_init`.
        plane: PLANE_LIMBO,

        motion: Motion::fixed(pos),
        anim: anim,
        facing: V3::new(1, 0, 0),
        target_velocity: scalar(0),
        appearance: appearance,

        stable_id: NO_STABLE_ID,
        attachment: EntityAttachment::World,
        child_inventories: HashSet::new(),
    };

    let eid = unwrap!(f.world_mut().entities.insert(e));
    post_init(f, eid);
    f.with_hooks(|h| h.on_entity_create(eid));
    Ok(eid)
}

pub fn create_unchecked<'d, F>(f: &mut F) -> EntityId
        where F: Fragment<'d> {
    let eid = f.world_mut().entities.insert(Entity {
        stable_plane: Stable::none(),
        plane: PLANE_LIMBO,

        motion: Motion::fixed(scalar(0)),
        anim: 0,
        facing: scalar(0),
        target_velocity: scalar(0),
        appearance: 0,

        stable_id: NO_STABLE_ID,
        attachment: EntityAttachment::World,
        child_inventories: HashSet::new(),
    }).unwrap();     // Shouldn't fail when stable_id == NO_STABLE_ID
    eid
}

pub fn post_init<'d, F>(f: &mut F,
                        eid: EntityId)
        where F: Fragment<'d> {
    let w = f.world_mut();
    let e = &mut w.entities[eid];

    e.plane = w.planes.get_id(e.stable_plane).unwrap_or(PLANE_LIMBO);
    trace!("looking for stable plane {:?} for entity {:?}: {:?}", e.stable_plane, eid, e.plane);
    if e.plane == PLANE_LIMBO {
        multimap_insert(&mut w.limbo_entities, e.stable_plane, eid);
    } else {
        multimap_insert(&mut w.entities_by_plane, e.plane, eid);
    }
}

pub fn pre_fini<'d, F>(f: &mut F,
                       eid: EntityId)
        where F: Fragment<'d> {
    let w = f.world_mut();
    let e = &w.entities[eid];

    if e.plane == PLANE_LIMBO {
        multimap_remove(&mut w.limbo_entities, e.stable_plane, eid);
    } else {
        multimap_remove(&mut w.entities_by_plane, e.plane, eid);
    }
}

pub fn destroy<'d, F>(f: &mut F,
                      eid: EntityId) -> OpResult<()>
        where F: Fragment<'d> {
    use world::EntityAttachment::*;
    pre_fini(f, eid);
    let e = unwrap!(f.world_mut().entities.remove(eid));
    // Further lookup failures indicate an invariant violation.

    match e.attachment {
        World => {},
        Chunk => {},
        Client(cid) => {
            // The parent Client may not exist due to `x_destroy` operating top-down.
            // (`client_destroy` destroys the Client first, then calls `entity_destroy` on each
            // child entity.  In this situation, `cid` will not be found in `w.clients`.)
            if let Some(c) = f.world_mut().clients.get_mut(cid) {
                if c.pawn == Some(eid) {
                    // NB: keep this behavior in sync with client_clear_pawn
                    c.pawn = None;
                }
                c.child_entities.remove(&eid);
            }
        },
    }

    for &iid in e.child_inventories.iter() {
        ops::inventory::destroy(f, iid).unwrap();
    }

    f.with_hooks(|h| h.on_entity_destroy(eid));
    Ok(())
}

pub fn attach<'d, F>(f: &mut F,
                     eid: EntityId,
                     new_attach: EntityAttachment) -> OpResult<EntityAttachment>
        where F: Fragment<'d> {
    use world::EntityAttachment::*;

    let w = f.world_mut();
    let e = unwrap!(w.entities.get_mut(eid));

    if new_attach == e.attachment {
        return Ok(new_attach);
    }

    match new_attach {
        World => {},
        Chunk => {
            fail!("EntityAttachment::Chunk is not yet supported");
            // TODO: check that e.motion is stationary
            /*
            let chunk_id = e.pos(0).reduce().div_floor(scalar(CHUNK_SIZE));
            unwrap!(w.terrain_chunks.get(&chunk_id),
                    "can't attach entity to unloaded chunk");
            */
        },
        Client(cid) => {
            let c = unwrap!(w.clients.get_mut(cid),
                            "can't attach entity to nonexistent client");
            c.child_entities.insert(eid);
        },
    }

    let old_attach = replace(&mut e.attachment, new_attach);

    // For `old_attach`, we assume that the chunk/client/etc exists, due to the World invariants.
    match old_attach {
        World => {},
        Chunk => {},    // No separate cache to update
        Client(cid) => {
            let c = &mut w.clients[cid];
            c.child_entities.remove(&eid);
        },
    }

    Ok(old_attach)
}

pub fn set_plane<'d, F>(f: &mut F,
                        eid: EntityId,
                        new_pid: PlaneId) -> OpResult<()>
        where F: Fragment<'d> {
    let new_stable_pid = unwrap!(f.world_mut().planes.try_pin(new_pid));
    set_stable_plane(f, eid, new_stable_pid)
}

pub fn set_stable_plane<'d, F>(f: &mut F,
                               eid: EntityId,
                               new_stable_pid: Stable<PlaneId>) -> OpResult<()>
        where F: Fragment<'d> {
    {
        let w = f.world_mut();
        let e = unwrap!(w.entities.get_mut(eid));

        let old_stable_pid = e.stable_plane;
        if new_stable_pid == old_stable_pid {
            return Ok(());
        }

        let new_pid = w.planes.get_id(new_stable_pid).unwrap_or(PLANE_LIMBO);
        let old_pid = e.plane;

        if old_pid == PLANE_LIMBO {
            multimap_remove(&mut w.limbo_entities, old_stable_pid, eid);
        } else {
            multimap_remove(&mut w.entities_by_plane, old_pid, eid);
        }

        if new_pid == PLANE_LIMBO {
            multimap_insert(&mut w.limbo_entities, new_stable_pid, eid);
        } else {
            multimap_insert(&mut w.entities_by_plane, new_pid, eid);
        }

        e.plane = new_pid;
        e.stable_plane = new_stable_pid;
    }

    f.with_hooks(|h| h.on_entity_plane_change(eid));
    Ok(())
}
