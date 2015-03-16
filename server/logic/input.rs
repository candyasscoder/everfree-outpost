use std::borrow::ToOwned;
use std::error::Error;

use physics::{CHUNK_SIZE, TILE_SIZE};

use types::*;
use util::{SmallSet, SmallVec};
use util::StrResult;

use chunks;
use engine::Engine;
use engine::glue::*;
use engine::split::{EngineRef, Part, Open};
use input::{Action, InputBits};
use messages::{ClientResponse, Dialog};
use physics_;
use script;
use terrain_gen;
use world::{self, World};
use world::object::*;
use world::save::{self, ObjectReader, ObjectWriter, ReadHooks, WriteHooks};
use vision::{self, vision_region};


pub fn input(mut eng: EngineRef, cid: ClientId, input: InputBits) {
    let now = eng.now();

    let target_velocity = input.to_velocity();
    if let Some(eid) = eng.world().get_client(cid).and_then(|c| c.pawn_id()) {
        warn_on_err!(physics_::Fragment::set_velocity(
                &mut eng.as_physics_fragment(), now, eid, target_velocity));
        let Open { world, messages, .. } = eng.open();
        let e = world.entity(eid);
        if e.motion().end_pos != e.motion().start_pos {
            messages.schedule_physics_update(eid, e.motion().end_time());
        }
    }
}

pub fn action(mut eng: EngineRef, cid: ClientId, action: Action) {
    match action {
        Action::Use => {
            unimplemented!()
        },
        Action::Inventory => {
            warn_on_err!(script::ScriptEngine::cb_open_inventory(eng.unwrap(), cid));
        },
        Action::UseItem(item_id) => {
            unimplemented!()
        },
    }
}

pub fn unsubscribe_inventory(mut eng: EngineRef, cid: ClientId, iid: InventoryId) {
    // No need for cherks - unsubscribe_inventory does nothing if the arguments are invalid.
    vision::Fragment::unsubscribe_inventory(&mut eng.as_vision_fragment(), cid, iid);
}



pub fn physics_update(mut eng: EngineRef, eid: EntityId) {
    let now = eng.now();

    let really_update =
        if let Some(e) = eng.world().get_entity(eid) {
            e.motion().end_time() <= now
        } else {
            false
        };

    if really_update {
        warn_on_err!(physics_::Fragment::update(&mut eng.as_physics_fragment(), now, eid));

        let Open { world, messages, .. } = eng.open();
        let e = world.entity(eid);
        if e.motion().end_pos != e.motion().start_pos {
            messages.schedule_physics_update(eid, e.motion().end_time());
        }
    }
}


pub fn open_inventory(mut eng: EngineRef, cid: ClientId, iid: InventoryId) -> StrResult<()> {
    // Check that IDs are valid.
    unwrap!(eng.world().get_client(cid));
    unwrap!(eng.world().get_inventory(iid));

    eng.messages_mut().send_client(cid, ClientResponse::OpenDialog(Dialog::Inventory(iid)));
    vision::Fragment::subscribe_inventory(&mut eng.as_vision_fragment(), cid, iid);

    Ok(())
}
