use types::*;
use util::SmallVec;

use engine::glue::*;
use engine::split::EngineRef;
use logic;
use messages::ClientResponse;
use world;
use world::object::*;
use world::save::{self, ObjectReader, ObjectWriter};
use vision::{self, vision_region};


pub fn register(mut eng: EngineRef, name: &str, appearance: u32) -> save::Result<()> {
    let pawn_id;
    let cid;

    {
        let mut eng = eng.as_hidden_world_fragment();

        let pos = V3::new(32, 32, 0);
        pawn_id = try!(world::Fragment::create_entity(&mut eng, pos, 2, appearance)).id();

        cid = {
            let mut c = try!(world::Fragment::create_client(&mut eng, name));
            try!(c.set_pawn(Some(pawn_id)));
            c.id()
        };
    }

    {
        let (h, eng) = eng.borrow().0.split_off();
        let h = SaveWriteHooks(h);
        let c = eng.world().client(cid);
        let file = eng.storage().create_client_file(c.name());
        let mut sw = ObjectWriter::new(file, h);
        try!(sw.save_client(&c));
    }
    try!(world::Fragment::destroy_client(&mut eng.as_hidden_world_fragment(), cid));

    Ok(())
}

pub fn login(mut eng: EngineRef, wire_id: WireId, name: &str) -> save::Result<()> {
    let now = eng.now();

    if let Some(old_cid) = eng.messages().name_to_client(name) {
        eng.borrow().unwrap().kick_client(old_cid, "logged in from another location");
    }

    // Load the client into the world.
    let cid =
        if let Some(file) = eng.storage().open_client_file(name) {
            let mut sr = ObjectReader::new(file);
            try!(sr.load_client(&mut eng.as_save_read_fragment()))
        } else {
            fail!("client file not found");
        };

    // Tell Vision about the client's entity (or entities).
    {
        let mut eids = SmallVec::new();
        for e in eng.world().client(cid).child_entities() {
            eids.push(e.id());
        }
        for &eid in eids.as_slice().iter() {
            let area = logic::world::entity_area(eng.world(), eid);
            vision::Fragment::add_entity(&mut eng.as_vision_fragment(), eid, area);
        }
    }

    // Load the chunks the client can currently see.
    let center = match eng.world().client(cid).pawn() {
        Some(eng) => eng.pos(now),
        None => scalar(0),
    };
    let region = vision::vision_region(center);

    for cpos in region.points() {
        logic::chunks::load_chunk(eng.borrow(), cpos);
    }

    // Set up the client to receive messages.
    info!("{:?}: logged in as {} ({:?})",
          wire_id, name, cid);
    eng.messages_mut().add_client(cid, wire_id, name);
    eng.messages_mut().schedule_check_view(cid, now + 1000);

    // Send the client's startup messages.
    if let Some(pawn_id) = eng.world().client(cid).pawn_id() {
        eng.messages_mut().send_client(cid, ClientResponse::Init(pawn_id));
    } else {
        warn!("{:?}: client has no pawn", cid);
    }

    vision::Fragment::add_client(&mut eng.as_vision_fragment(), cid, region);

    Ok(())
}

pub fn logout(mut eng: EngineRef, cid: ClientId) -> save::Result<()> {
    eng.messages_mut().remove_client(cid);

    let old_region = eng.vision().client_view_area(cid);
    vision::Fragment::remove_client(&mut eng.as_vision_fragment(), cid);
    if let Some(old_region) = old_region {
        for cpos in old_region.points() {
            logic::chunks::unload_chunk(eng.borrow(), cpos);
        }
    }

    {
        let (h, eng) = eng.borrow().0.split_off();
        let h = SaveWriteHooks(h);
        let c = eng.world().client(cid);
        let file = eng.storage().create_client_file(c.name());
        let mut sw = ObjectWriter::new(file, h);
        try!(sw.save_client(&c));
    }
    try!(world::Fragment::destroy_client(&mut eng.as_world_fragment(), cid));
    Ok(())
}

pub fn update_view(mut eng: EngineRef, cid: ClientId) {
    let now = eng.now();

    let old_region = match eng.vision().client_view_area(cid) {
        Some(x) => x,
        None => return,
    };

    let new_region = {
        // TODO: warn on None? - may indicate inconsistency between World and Vision
        let client = unwrap_or!(eng.world().get_client(cid));

        // TODO: make sure return is the right thing to do on None
        let pawn = unwrap_or!(client.pawn());

        vision::vision_region(pawn.pos(now))
    };

    // un/load_chunk use HiddenWorldFragment, so do the calls in this specific order to make sure
    // the chunks being un/loaded are actually not in the client's vision.

    for cpos in new_region.points().filter(|&p| !old_region.contains(p)) {
        logic::chunks::load_chunk(eng.borrow(), cpos);
    }

    vision::Fragment::set_client_view(&mut eng.as_vision_fragment(), cid, new_region);

    for cpos in old_region.points().filter(|&p| !new_region.contains(p)) {
        logic::chunks::unload_chunk(eng.borrow(), cpos);
    }

    eng.messages_mut().schedule_check_view(cid, now + 1000);
}
