use std::borrow::ToOwned;
use std::error::Error;

use physics::{CHUNK_SIZE, TILE_SIZE};

use types::*;
use util::{SmallSet, SmallVec};
use util::StrResult;

use chunks;
use engine::Engine;
use engine::glue::*;
use engine::split::{EngineRef, Part};
use input::{Action, InputBits};
use logic;
use messages::{ClientResponse, Dialog};
use physics_;
use script;
use terrain_gen;
use world::{self, World};
use world::object::*;
use world::save::{self, ObjectReader, ObjectWriter, ReadHooks, WriteHooks};
use vision::{self, vision_region};


pub fn register(eng: &mut Engine, name: &str, appearance: u32) -> save::Result<()> {
    let pawn_id;
    let cid;

    {
        let mut eng: HiddenWorldFragment = EngineRef::new(eng).slice();

        pawn_id = try!(world::Fragment::create_entity(&mut eng, scalar(0), 2, appearance)).id();

        cid = {
            let mut c = try!(world::Fragment::create_client(&mut eng, name));
            try!(c.set_pawn(Some(pawn_id)));
            c.id()
        };
    }

    {
        let (h, eng): (SaveWriteHooks, _) = EngineRef::new(eng).split_off();
        let c = eng.world().client(cid);
        let file = eng.storage().create_client_file(c.name());
        let mut sw = ObjectWriter::new(file, h);
        try!(sw.save_client(&c));
    }
    {
        let mut eng: HiddenWorldFragment = EngineRef::new(eng).slice();
        try!(world::Fragment::destroy_client(&mut eng, cid));
    }

    Ok(())
}

pub fn login(eng: &mut Engine, wire_id: WireId, name: &str) -> save::Result<()> {
    let now = eng.now;

    // Load the client into the world.
    let cid =
        if let Some(file) = eng.storage.open_client_file(name) {
            let mut eng: SaveReadFragment = EngineRef::new(eng).slice();
            let mut sr = ObjectReader::new(file);
            try!(sr.load_client(&mut eng))
        } else {
            fail!("client file not found");
        };

    // Tell Vision about the client's entity (or entities).
    {
        let mut eids = SmallVec::new();
        for e in eng.world.client(cid).child_entities() {
            eids.push(e.id());
        }
        let (mut h, mut eng): (VisionHooks, _) = EngineRef::new(eng).split_off();
        for &eid in eids.as_slice().iter() {
            let area = logic::world::entity_area(h.world(), eid);
            eng.vision_mut().add_entity(eid, area, &mut h);
        }
    }

    // Load the chunks the client can currently see.
    let center = match eng.world.client(cid).pawn() {
        Some(eng) => eng.pos(now),
        None => scalar(0),
    };
    let region = vision::vision_region(center);

    for cpos in region.points() {
        logic::chunks::load_chunk(eng, cpos);
    }

    // Set up the client to receive messages.
    info!("{:?}: logged in as {} ({:?})",
          wire_id, name, cid);
    eng.messages.add_client(cid, wire_id);
    eng.messages.schedule_check_view(cid, now + 1000);

    // Send the client's startup messages.
    if let Some(pawn_id) = eng.world.client(cid).pawn_id() {
        eng.messages.send_client(cid, ClientResponse::Init(pawn_id));
    } else {
        warn!("{:?}: client has no pawn", cid);
    }

    {
        let (mut h, mut eng): (VisionHooks, _) = EngineRef::new(eng).split_off();
        eng.vision_mut().add_client(cid, region, &mut h);
    }

    Ok(())
}

pub fn logout(eng: &mut Engine, cid: ClientId) -> save::Result<()> {
    eng.messages.remove_client(cid);

    {
        let (mut h, mut eng): (VisionHooks, _) = EngineRef::new(eng).split_off();
        eng.vision_mut().remove_client(cid, &mut h);
    }
    if let Some(old_region) = eng.vision.client_view_area(cid) {
        for cpos in old_region.points() {
            logic::chunks::unload_chunk(eng, cpos);
        }
    }

    {
        let (h, eng): (SaveWriteHooks, _) = EngineRef::new(eng).split_off();
        let c = eng.world().client(cid);
        let file = eng.storage().create_client_file(c.name());
        let mut sw = ObjectWriter::new(file, h);
        try!(sw.save_client(&c));
    }
    {
        let mut eng: WorldFragment = EngineRef::new(eng).slice();
        try!(world::Fragment::destroy_client(&mut eng, cid));
    }
    Ok(())
}

pub fn update_view(eng: &mut Engine, cid: ClientId) {
    let now = eng.now;

    let old_region = match eng.vision.client_view_area(cid) {
        Some(x) => x,
        None => return,
    };

    let new_region = {
        // TODO: warn on None? - may indicate inconsistency between World and Vision
        let client = unwrap_or!(eng.world.get_client(cid));

        // TODO: make sure return is the right thing to do on None
        let pawn = unwrap_or!(client.pawn());

        vision::vision_region(pawn.pos(now))
    };

    for cpos in old_region.points().filter(|&p| !new_region.contains(p)) {
        logic::chunks::unload_chunk(eng, cpos);
    }

    for cpos in new_region.points().filter(|&p| !old_region.contains(p)) {
        logic::chunks::load_chunk(eng, cpos);
    }

    {
        let (mut h, mut eng): (VisionHooks, _) = EngineRef::new(eng).split_off();
        eng.vision_mut().set_client_view(cid, new_region, &mut h);
    }

    eng.messages.schedule_check_view(cid, now + 1000);
}
