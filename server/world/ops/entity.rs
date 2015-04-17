use std::collections::HashSet;
use std::mem::replace;

use types::*;
use util::stable_id_map::NO_STABLE_ID;

use world::{Entity, EntityAttachment, Motion};
use world::{Fragment, Hooks};
use world::ops::{self, OpResult};


pub fn create<'d, F>(f: &mut F,
                     stable_pid: Stable<PlaneId>,
                     pos: V3,
                     anim: AnimId,
                     appearance: u32) -> OpResult<EntityId>
        where F: Fragment<'d> {
    let pid = f.world().planes.get_id(stable_pid).unwrap_or(PLANE_LIMBO);

    let e = Entity {
        stable_plane: stable_pid,
        plane: pid,

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

pub fn destroy<'d, F>(f: &mut F,
                      eid: EntityId) -> OpResult<()>
        where F: Fragment<'d> {
    use world::EntityAttachment::*;
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
