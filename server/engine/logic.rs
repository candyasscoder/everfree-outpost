use types::*;

use chunks;
use engine::Engine;
use engine::glue::EngineRef;
use engine::hooks::{WorldHooks, VisionHooks};
use messages::ClientResponse;
use script::{ReadHooks, WriteHooks};
use world::WorldMut;
use world::object::*;
use world::save::{self, ObjectReader, ObjectWriter};
use vision;

pub fn register(e: &mut Engine, name: &str, appearance: u32) -> save::Result<()> {
    let mut h = WorldHooks {
        now: 0,
        vision: &mut e.vision,
        messages: &mut e.messages,
    };
    let mut w = e.world.hook(&mut h);

    let pawn_id = try!(w.create_entity(scalar(0), 2, appearance)).id();

    let cid = {
        let mut c = try!(w.create_client(name));
        try!(c.set_pawn(Some(pawn_id)));
        c.id()
    };

    {
        let c = w.world().client(cid);
        let file = e.storage.create_client_file(c.name());
        let mut sw = ObjectWriter::new(file, WriteHooks::new(&mut e.script));
        try!(sw.save_client(&c));
    }
    try!(w.destroy_client(cid));

    Ok(())
}

pub fn login(e: &mut Engine, now: Time, wire_id: WireId, name: &str) -> save::Result<()> {
    // Load the client into the world.
    let cid =
        if let Some(file) = e.storage.open_client_file(name) {
            let mut h = WorldHooks {
                now: 0,
                vision: &mut e.vision,
                messages: &mut e.messages,
            };
            let mut w = e.world.hook(&mut h);

            let mut sr = ObjectReader::new(file, ReadHooks::new(&mut e.script));
            try!(sr.load_client(&mut w))
        } else {
            fail!("client file not found");
        };

    // Load the chunks the client can currently see.
    let center = match e.world.client(cid).pawn() {
        Some(e) => e.pos(now),
        None => scalar(0),
    };
    let region = vision::vision_region(center);

    for cpos in region.points() {
        chunks::Fragment::load(&mut EngineRef(e), cpos);
    }

    // Set up the client to receive messages.
    info!("{:?}: logged in as {} ({:?})",
          wire_id, name, cid);
    e.messages.add_client(cid, wire_id);

    // Send the client's startup messages.
    if let Some(pawn_id) = e.world.client(cid).pawn_id() {
        e.messages.send_client(cid, ClientResponse::Init(pawn_id));
    } else {
        warn!("{:?}: client has no pawn", cid);
    }

    e.vision.add_client(cid,
                        region,
                        &mut VisionHooks {
                            messages: &mut e.messages,
                            world: &e.world,
                        });

    Ok(())
}

pub fn logout(e: &mut Engine, cid: ClientId) -> save::Result<()> {
    {
        let c = e.world.client(cid);
        let file = e.storage.create_client_file(c.name());
        let mut sw = ObjectWriter::new(file, WriteHooks::new(&mut e.script));
        try!(sw.save_client(&c));
    }

    let mut h = WorldHooks {
        now: 0,
        vision: &mut e.vision,
        messages: &mut e.messages,
    };
    let mut w = e.world.hook(&mut h);
    try!(w.destroy_client(cid));
    Ok(())
}
