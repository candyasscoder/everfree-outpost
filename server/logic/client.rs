use std::borrow::ToOwned;

use types::*;
use util::SmallVec;

use chunks;
use engine::glue::*;
use engine::split::EngineRef;
use logic;
use messages::ClientResponse;
use script;
use world;
use world::object::*;
use world::save::{self, ObjectReader, ObjectWriter};
use vision::{self, vision_region};


const DAY_NIGHT_CYCLE_TICKS: u32 = 24_000;
const DAY_NIGHT_CYCLE_MS: u32 = 24 * 60 * 1000;

pub fn register(mut eng: EngineRef, name: &str, appearance: u32) -> save::Result<()> {
    let pawn_id;
    let cid;

    {
        let mut eng = eng.as_hidden_world_fragment();

        let pos = V3::new(32, 32, 0);
        let pid = STABLE_PLANE_FOREST;
        pawn_id = try!(world::Fragment::create_entity(&mut eng, pid, pos, 2, appearance)).id();

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
    // NB: SaveReadFragment uses HiddenWorldFragment, which means it does not send updates to
    // clients.
    let cid =
        if let Some(file) = eng.storage().open_client_file(name) {
            let mut sr = ObjectReader::new(file);
            try!(sr.load_client(&mut eng.as_save_read_fragment(), name.to_owned()))
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
            let (pid, area) = {
                let e = eng.world().entity(eid);
                (e.plane_id(), logic::world::entity_area(e))
            };
            // TODO: This is kind of a hack.  The entity was loaded using HiddenVisionFragment, so
            // clients were not notified.  We remove and re-add the entity using the normal
            // VisionFragment so that other clients can see the new entity.
            vision::Fragment::remove_entity(&mut eng.as_vision_fragment(), eid);
            vision::Fragment::add_entity(&mut eng.as_vision_fragment(), eid, pid, area);
        }
    }

    // Load the chunks the client can currently see.
    let (pawn_stable_pid, center) = match eng.world().client(cid).pawn() {
        Some(e) => (e.stable_plane_id(), e.pos(now)),
        None => (STABLE_PLANE_LIMBO, scalar(0)),
    };
    let region = vision::vision_region(center);

    let pawn_pid = chunks::Fragment::get_plane_id(&mut eng.as_chunks_fragment(), pawn_stable_pid);
    for cpos in region.points() {
        logic::chunks::load_chunk(eng.borrow(), pawn_pid, cpos);
    }

    // Set up the client to receive messages.
    info!("{:?}: logged in as {} ({:?})",
          wire_id, name, cid);
    eng.messages_mut().add_client(cid, wire_id, name);

    // Send the client's startup messages.
    let opt_eid = eng.world().client(cid).pawn_id();
    let cycle_base = (now % DAY_NIGHT_CYCLE_MS as Time) as u32;
    eng.messages_mut().send_client(cid, ClientResponse::Init(opt_eid,
                                                             now,
                                                             cycle_base,
                                                             DAY_NIGHT_CYCLE_MS));

    vision::Fragment::add_client(&mut eng.as_vision_fragment(), cid, pawn_pid, region);

    warn_on_err!(script::ScriptEngine::cb_login(eng.unwrap(), cid));

    Ok(())
}

pub fn logout(mut eng: EngineRef, cid: ClientId) -> save::Result<()> {
    eng.messages_mut().remove_client(cid);

    let old_region = eng.vision().client_view_area(cid);
    let old_pid = eng.vision().client_view_plane(cid);
    vision::Fragment::remove_client(&mut eng.as_vision_fragment(), cid);
    if let (Some(old_region), Some(old_pid)) = (old_region, old_pid) {
        for cpos in old_region.points() {
            logic::chunks::unload_chunk(eng.borrow(), old_pid, cpos);
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

    let old_region = unwrap_or!(eng.vision().client_view_area(cid));
    let old_pid = unwrap_or!(eng.vision().client_view_plane(cid));

    let (new_stable_pid, new_region, pawn_id) = {
        // TODO: warn on None? - may indicate inconsistency between World and Vision
        let client = unwrap_or!(eng.world().get_client(cid));

        // TODO: make sure return is the right thing to do on None
        let pawn = unwrap_or!(client.pawn());

        (pawn.stable_plane_id(),
         vision::vision_region(pawn.pos(now)),
         pawn.id())
    };
    let new_pid = chunks::Fragment::get_plane_id(&mut eng.as_chunks_fragment(), new_stable_pid);

    let plane_change = new_pid != old_pid;

    // un/load_chunk use HiddenWorldFragment, so do the calls in this specific order to make sure
    // the chunks being un/loaded are actually not in the client's vision.

    for cpos in new_region.points().filter(|&p| !old_region.contains(p) || plane_change) {
        logic::chunks::load_chunk(eng.borrow(), new_pid, cpos);
    }

    vision::Fragment::set_client_view(&mut eng.as_vision_fragment(), cid, new_pid, new_region);

    for cpos in old_region.points().filter(|&p| !new_region.contains(p) || plane_change) {
        logic::chunks::unload_chunk(eng.borrow(), old_pid, cpos);
    }

    // TODO: using `with_hooks` here is gross, move schedule_view_update somewhere better
    {
        use world::fragment::Fragment;
        eng.as_world_fragment().with_hooks(|h| h.schedule_view_update(pawn_id));
    }
}
