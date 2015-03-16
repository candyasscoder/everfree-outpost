use std::borrow::ToOwned;
use std::error::Error;

use physics::{CHUNK_SIZE, TILE_SIZE};

use types::*;
use util::{SmallSet, SmallVec};
use util::StrResult;

use chunks;
use engine::Engine;
use engine::glue::*;
use engine::split::EngineRef;
use input::{Action, InputBits};
use messages::{ClientResponse, Dialog};
use physics_;
use script;
use terrain_gen;
use world::{self, World};
use world::object::*;
use world::save::{self, ObjectReader, ObjectWriter, ReadHooks, WriteHooks};
use vision::{self, vision_region};


pub fn input(eng: &mut Engine, cid: ClientId, input: InputBits) {
    let now = eng.now;

    let target_velocity = input.to_velocity();
    if let Some(eid) = eng.world.get_client(cid).and_then(|c| c.pawn_id()) {
        {
            let mut eng: PhysicsFragment = EngineRef::new(eng).slice();
            warn_on_err!(physics_::Fragment::set_velocity(
                    &mut eng, now, eid, target_velocity));
        }
        let e = eng.world.entity(eid);
        if e.motion().end_pos != e.motion().start_pos {
            eng.messages.schedule_physics_update(eid, e.motion().end_time());
        }
    }
}

pub fn action(eng: &mut Engine, cid: ClientId, action: Action) {
    match action {
        Action::Use => {
            unimplemented!()
        },
        Action::Inventory => {
            warn_on_err!(script::ScriptEngine::cb_open_inventory(eng, cid));
        },
        Action::UseItem(item_id) => {
            unimplemented!()
        },
    }
}

pub fn unsubscribe_inventory(eng: &mut Engine, cid: ClientId, iid: InventoryId) {
    // No need for cherks - unsubscribe_inventory does nothing if the arguments are invalid.
    let (mut h, mut eng): (VisionHooks, _) = EngineRef::new(eng).split_off();
    eng.vision_mut().unsubscribe_inventory(cid, iid, &mut h);
}



pub fn physics_update(eng: &mut Engine, eid: EntityId) {
    let now = eng.now;

    let really_update =
        if let Some(e) = eng.world.get_entity(eid) {
            e.motion().end_time() <= now
        } else {
            false
        };

    if really_update {
        {
            let mut eng: PhysicsFragment = EngineRef::new(eng).slice();
            warn_on_err!(physics_::Fragment::update(&mut eng, now, eid));
        }

        let e = eng.world.entity(eid);
        if e.motion().end_pos != e.motion().start_pos {
            eng.messages.schedule_physics_update(eid, e.motion().end_time());
        }
    }
}


pub fn open_inventory(eng: &mut Engine, cid: ClientId, iid: InventoryId) -> StrResult<()> {
    // Check that IDs are valid.
    unwrap!(eng.world.get_client(cid));
    unwrap!(eng.world.get_inventory(iid));

    eng.messages.send_client(cid, ClientResponse::OpenDialog(Dialog::Inventory(iid)));
    {
        let (mut h, mut eng): (VisionHooks, _) = EngineRef::new(eng).split_off();
        eng.vision_mut().subscribe_inventory(cid, iid, &mut h);
    }

    Ok(())
}
