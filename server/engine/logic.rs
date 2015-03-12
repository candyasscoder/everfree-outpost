use std::borrow::ToOwned;

use physics::{CHUNK_SIZE, TILE_SIZE};

use types::*;

use chunks;
use engine::Engine;
use engine::glue::*;
use engine::split::EngineRef;
use messages::ClientResponse;
use util::SmallSet;
use world::{self, World};
use world::object::*;
use world::save::{self, ObjectReader, ObjectWriter};
use vision::{self, vision_region};


pub fn register(e: &mut Engine, name: &str, appearance: u32) -> save::Result<()> {
    let pawn_id;
    let cid;

    {
        let mut e: WorldFragment = EngineRef::new(e).slice();

        pawn_id = try!(world::Fragment::create_entity(&mut e, scalar(0), 2, appearance)).id();

        cid = {
            let mut c = try!(world::Fragment::create_client(&mut e, name));
            try!(c.set_pawn(Some(pawn_id)));
            c.id()
        };
    }

    {
        let (h, e): (SaveWriteHooks, _) = EngineRef::new(e).split_off();
        let c = e.world().client(cid);
        let file = e.storage().create_client_file(c.name());
        let mut sw = ObjectWriter::new(file, h);
        try!(sw.save_client(&c));
    }
    {
        let mut e: WorldFragment = EngineRef::new(e).slice();
        try!(world::Fragment::destroy_client(&mut e, cid));
    }

    Ok(())
}

pub fn login(e: &mut Engine, now: Time, wire_id: WireId, name: &str) -> save::Result<()> {
    // Load the client into the world.
    let cid =
        if let Some(file) = e.storage.open_client_file(name) {
            let mut e: SaveReadFragment = EngineRef::new(e).slice();
            let mut sr = ObjectReader::new(file);
            try!(sr.load_client(&mut e))
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
        let mut e: ChunksFragment = EngineRef::new(e).slice();
        chunks::Fragment::load(&mut e, cpos);
    }

    // Set up the client to receive messages.
    info!("{:?}: logged in as {} ({:?})",
          wire_id, name, cid);
    e.messages.add_client(cid, wire_id);
    e.messages.schedule_check_view(cid, now + 1000);

    // Send the client's startup messages.
    if let Some(pawn_id) = e.world.client(cid).pawn_id() {
        e.messages.send_client(cid, ClientResponse::Init(pawn_id));
    } else {
        warn!("{:?}: client has no pawn", cid);
    }

    {
        let (mut h, mut e): (VisionHooks, _) = EngineRef::new(e).split_off();
        e.vision_mut().add_client(cid, region, &mut h);
    }

    Ok(())
}

pub fn logout(e: &mut Engine, cid: ClientId) -> save::Result<()> {
    {
        let (h, e): (SaveWriteHooks, _) = EngineRef::new(e).split_off();
        let c = e.world().client(cid);
        let file = e.storage().create_client_file(c.name());
        let mut sw = ObjectWriter::new(file, h);
        try!(sw.save_client(&c));
    }
    {
        let mut e: WorldFragment = EngineRef::new(e).slice();
        try!(world::Fragment::destroy_client(&mut e, cid));
    }
    Ok(())
}


pub fn update_view(e: &mut Engine, now: Time, cid: ClientId) {
    let old_region = match e.vision.client_view_area(cid) {
        Some(x) => x,
        None => return,
    };

    let new_region = {
        // TODO: warn on None? - may indicate inconsistency between World and Vision
        let client = unwrap_or!(e.world.get_client(cid));

        // TODO: make sure return is the right thing to do on None
        let pawn = unwrap_or!(client.pawn());

        vision::vision_region(pawn.pos(now))
    };

    for cpos in old_region.points().filter(|&p| !new_region.contains(p)) {
        let mut e: ChunksFragment = EngineRef::new(e).slice();
        chunks::Fragment::unload(&mut e, cpos);
    }

    for cpos in new_region.points().filter(|&p| !old_region.contains(p)) {
        let mut e: ChunksFragment = EngineRef::new(e).slice();
        chunks::Fragment::load(&mut e, cpos);
    }

    {
        let (mut h, mut e): (VisionHooks, _) = EngineRef::new(e).split_off();
        e.vision_mut().add_client(cid, new_region, &mut h);
    }
}


impl<'a, 'd> world::Hooks for WorldHooks<'a, 'd> {
    fn on_client_create(&mut self, cid: ClientId) {
    }

    fn on_client_destroy(&mut self, cid: ClientId) {
        let (mut h, mut e): (VisionHooks, _) = self.borrow().split_off();
        e.vision_mut().remove_client(cid, &mut h);
    }

    fn on_client_change_pawn(&mut self,
                             cid: ClientId,
                             old_pawn: Option<EntityId>,
                             new_pawn: Option<EntityId>) {
        let now = self.now();
        let center = match self.world().client(cid).pawn() {
            Some(e) => e.pos(now),
            None => scalar(0),
        };

        let (mut h, mut e): (VisionHooks, _) = self.borrow().split_off();
        e.vision_mut().set_client_view(cid, vision_region(center), &mut h);
    }


    fn on_terrain_chunk_create(&mut self, pos: V2) {
        let (mut h, mut e): (VisionHooks, _) = self.borrow().split_off();
        e.vision_mut().add_chunk(pos, &mut h);
    }

    fn on_terrain_chunk_destroy(&mut self, pos: V2) {
        let (mut h, mut e): (VisionHooks, _) = self.borrow().split_off();
        e.vision_mut().remove_chunk(pos, &mut h);
    }

    fn on_chunk_invalidate(&mut self, pos: V2) {
        let (mut h, mut e): (VisionHooks, _) = self.borrow().split_off();
        e.vision_mut().update_chunk(pos, &mut h);
    }


    fn on_entity_create(&mut self, eid: EntityId) {
        let area = entity_area(self.world(), eid);
        let (mut h, mut e): (VisionHooks, _) = self.borrow().split_off();
        e.vision_mut().add_entity(eid, area, &mut h);
    }

    fn on_entity_destroy(&mut self, eid: EntityId) {
        let (mut h, mut e): (VisionHooks, _) = self.borrow().split_off();
        e.vision_mut().remove_entity(eid, &mut h);
    }

    fn on_entity_motion_change(&mut self, eid: EntityId) {
        let area = entity_area(self.world(), eid);
        let (mut h, mut e): (VisionHooks, _) = self.borrow().split_off();
        e.vision_mut().set_entity_area(eid, area, &mut h);
    }
}

fn entity_area(w: &World, eid: EntityId) -> SmallSet<V2> {
    let e = w.entity(eid);
    let mut area = SmallSet::new();

    let a = e.motion().start_pos.reduce().div_floor(scalar(CHUNK_SIZE * TILE_SIZE));
    let b = e.motion().end_pos.reduce().div_floor(scalar(CHUNK_SIZE * TILE_SIZE));

    area.insert(a);
    area.insert(b);
    area
}


impl<'a, 'd> vision::Hooks for VisionHooks<'a, 'd> {
    fn on_chunk_update(&mut self, cid: ClientId, pos: V2) {
        use util::encode_rle16;
        let tc = self.world().terrain_chunk(pos);
        let data = encode_rle16(tc.blocks().iter().map(|&x| x));
        self.messages().send_client(cid, ClientResponse::TerrainChunk(pos, data));
    }

    fn on_entity_appear(&mut self, cid: ClientId, eid: EntityId) {
        let entity = self.world().entity(eid);

        let appearance = entity.appearance();
        // TODO: hack.  Should have a separate "entity name" field somewhere.
        let name =
            if let world::EntityAttachment::Client(controller_cid) = entity.attachment() {
                self.world().client(controller_cid).name().to_owned()
            } else {
                String::new()
            };

        self.messages().send_client(cid, ClientResponse::EntityAppear(eid, appearance, name));
    }

    fn on_entity_disappear(&mut self, cid: ClientId, eid: EntityId) {
        let time =
            if let Some(entity) = self.world().get_entity(eid) {
                entity.motion().start_time
            } else {
                0
            };
        // TODO: figure out if it's actually useful to send the time here.  The client currently
        // ignores it.
        self.messages().send_client(cid, ClientResponse::EntityGone(eid, time));
    }

    fn on_entity_update(&mut self, cid: ClientId, eid: EntityId) {
        let entity = self.world().entity(eid);

        let motion = entity.motion().clone();
        let anim = entity.anim();
        self.messages().send_client(cid, ClientResponse::EntityUpdate(eid, motion, anim));
    }

}


impl<'a, 'd> chunks::Hooks for ChunksHooks<'a, 'd> {
}

impl<'a, 'd> chunks::Provider for ChunkProvider<'a, 'd> {
    type E = save::Error;

    fn load(&mut self, cpos: V2) -> save::Result<()> {
        if let Some(file) = self.storage().open_terrain_chunk_file(cpos) {
            let mut e: SaveReadFragment = self.borrow().slice();
            let mut sr = ObjectReader::new(file);
            try!(sr.load_terrain_chunk(&mut e));
        } else {
            let mut e: WorldFragment = self.borrow().slice();
            let id = e.data().block_data.get_id("grass/center/v0");
            let mut blocks = [0; 4096];
            for i in range(0, 256) {
                blocks[i] = id;
            }
            try!(world::Fragment::create_terrain_chunk(&mut e, cpos, Box::new(blocks)));
        }
        Ok(())
    }

    fn unload(&mut self, cpos: V2) -> save::Result<()> {
        {
            let (h, e): (SaveWriteHooks, _) = self.borrow().split_off();
            let t = e.world().terrain_chunk(cpos);
            let file = e.storage().create_terrain_chunk_file(cpos);
            let mut sw = ObjectWriter::new(file, h);
            try!(sw.save_terrain_chunk(&t));
        }
        {
            let mut e: WorldFragment = self.borrow().slice();
            try!(world::Fragment::destroy_terrain_chunk(&mut e, cpos));
        }
        Ok(())
    }
}


impl<'a, 'd> world::save::ReadHooks for SaveReadHooks<'a, 'd> {
}

impl<'a, 'd> world::save::WriteHooks for SaveWriteHooks<'a, 'd> {
}
