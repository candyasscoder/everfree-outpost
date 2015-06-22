use types::*;

use engine::split::{EngineRef, Open};
use input::{InputBits};
use messages::ClientResponse;
use msg::ExtraArg;
use physics_;
use script;
use world::object::*;
use vision;


pub fn input(mut eng: EngineRef, cid: ClientId, input: InputBits) {
    let now = eng.now();

    let target_velocity = input.to_velocity();
    if let Some(eid) = eng.world().get_client(cid).and_then(|c| c.pawn_id()) {
        warn_on_err!(physics_::Fragment::set_velocity(
                &mut eng.as_physics_fragment(), now, eid, target_velocity));
        let Open { world, timer, .. } = eng.open();
        let e = world.entity(eid);
        if e.motion().end_pos != e.motion().start_pos {
            timer.schedule(e.motion().end_time(),
                           move |eng| physics_update(eng, eid));
        }
    }
}

pub fn interact(eng: EngineRef, cid: ClientId, args: Option<ExtraArg>) {
    warn_on_err!(script::ScriptEngine::cb_interact(eng.unwrap(), cid, args));
}

pub fn use_item(eng: EngineRef, cid: ClientId, item_id: ItemId, args: Option<ExtraArg>) {
    warn_on_err!(script::ScriptEngine::cb_use_item(eng.unwrap(), cid, item_id, args));
}

pub fn use_ability(eng: EngineRef, cid: ClientId, item_id: ItemId, args: Option<ExtraArg>) {
    warn_on_err!(script::ScriptEngine::cb_use_ability(eng.unwrap(), cid, item_id, args));
}

pub fn open_inventory(eng: EngineRef, cid: ClientId) {
    warn_on_err!(script::ScriptEngine::cb_open_inventory(eng.unwrap(), cid));
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

        let Open { world, timer, .. } = eng.open();
        let e = world.entity(eid);
        if e.motion().end_pos != e.motion().start_pos {
            timer.schedule(e.motion().end_time(),
                           move |eng| physics_update(eng, eid));
        }
    }
}


pub fn chat(mut eng: EngineRef, cid: ClientId, msg: String) {
    // TODO: move this into a script
    if &*msg == "/count" {
        let count = eng.messages().clients_len();
        let msg_out = format!("***\t{} player{} online",
                              count,
                              if count != 1 { "s" } else { "" });
        eng.messages_mut().send_client(cid, ClientResponse::ChatUpdate(msg_out));
    } else if msg.starts_with("/") {
        warn_on_err!(script::ScriptEngine::cb_chat_command(eng.unwrap(), cid, &*msg));
    } else {
        if msg.len() > 400 {
            warn!("{:?}: bad request: chat message too long ({})", cid, msg.len());
            return;
        }

        let msg_out = format!("<{}>\t{}",
                              eng.world().client(cid).name(),
                              msg);
        eng.messages_mut().broadcast_clients(ClientResponse::ChatUpdate(msg_out));
    }
}
