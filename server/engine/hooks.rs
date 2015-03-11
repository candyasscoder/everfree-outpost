use std::borrow::ToOwned;

use physics::{CHUNK_SIZE, TILE_SIZE};
use types::*;
use util::SmallSet;

use engine::split::*;
use messages::ClientResponse;
use messages::Messages;
use vision::{self, Vision, vision_region};
use world::{self, World};
use world::object::*;



engine_part_typedef!(pub WorldHooksPart(vision, messages));

pub struct WorldHooks<'a, 'd> {
    pub now: Time,
    pub e: WorldHooksPart<'a, 'd>,
}

impl<'a, 'd> WorldHooks<'a, 'd> {
    pub fn new(now: Time, e: WorldHooksPart<'a, 'd>) -> WorldHooks<'a, 'd> {
        WorldHooks {
            now: now,
            e: e,
        }
    }
}




pub struct VisionHooks<'a, 'd: 'a> {
    pub messages: &'a mut Messages,
    pub world: &'a World<'d>,
}

impl<'a, 'd> VisionHooks<'a, 'd> {
    pub fn new(messages: &'a mut Messages, world: &'a World<'d>) -> VisionHooks<'a, 'd> {
        VisionHooks {
            messages: messages,
            world: world,
        }
    }
}

macro_rules! VisionHooks_new {
    ($owner:expr, $world:expr) => {
        $crate::engine::hooks::VisionHooks {
            messages: &mut $owner.messages,
            world: $world,
        }
    };
}


impl<'a, 'd> world::Hooks for WorldHooks<'a, 'd> {
    fn on_client_create(&mut self, w: &World, cid: ClientId) {
    }

    fn on_client_destroy(&mut self, w: &World, cid: ClientId) {
        let Open { vision, messages, .. } = self.e.open();
        vision.remove_client(cid, &mut VisionHooks::new(messages, w));
    }

    fn on_client_change_pawn(&mut self,
                             w: &World,
                             cid: ClientId,
                             old_pawn: Option<EntityId>,
                             new_pawn: Option<EntityId>) {
        let Open { vision, messages, .. } = self.e.open();
        let center = match w.client(cid).pawn() {
            Some(e) => e.pos(self.now),
            None => scalar(0),
        };
        vision.set_client_view(cid,
                               vision_region(center),
                               &mut VisionHooks::new(messages, w));
    }


    fn on_terrain_chunk_create(&mut self, w: &World, pos: V2) {
        let Open { vision, messages, .. } = self.e.open();
        vision.add_chunk(pos, &mut VisionHooks::new(messages, w));
    }

    fn on_terrain_chunk_destroy(&mut self, w: &World, pos: V2) {
        let Open { vision, messages, .. } = self.e.open();
        vision.remove_chunk(pos, &mut VisionHooks::new(messages, w));
    }

    fn on_chunk_invalidate(&mut self, w: &World, pos: V2) {
        let Open { vision, messages, .. } = self.e.open();
        vision.update_chunk(pos, &mut VisionHooks::new(messages, w));
    }


    fn on_entity_create(&mut self, w: &World, eid: EntityId) {
        let Open { vision, messages, .. } = self.e.open();
        vision.add_entity(eid,
                          entity_area(w, eid),
                          &mut VisionHooks::new(messages, w));
    }

    fn on_entity_destroy(&mut self, w: &World, eid: EntityId) {
        let Open { vision, messages, .. } = self.e.open();
        vision.remove_entity(eid, &mut VisionHooks::new(messages, w));
    }

    fn on_entity_motion_change(&mut self, w: &World, eid: EntityId) {
        let Open { vision, messages, .. } = self.e.open();
        vision.set_entity_area(eid,
                               entity_area(w, eid),
                               &mut VisionHooks::new(messages, w));
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
        let tc = self.world.terrain_chunk(pos);
        let data = encode_rle16(tc.blocks().iter().map(|&x| x));
        self.messages.send_client(cid, ClientResponse::TerrainChunk(pos, data));
    }

    fn on_entity_appear(&mut self, cid: ClientId, eid: EntityId) {
        let entity = self.world.entity(eid);

        let appearance = entity.appearance();
        // TODO: hack.  Should have a separate "entity name" field somewhere.
        let name =
            if let world::EntityAttachment::Client(controller_cid) = entity.attachment() {
                self.world.client(controller_cid).name().to_owned()
            } else {
                String::new()
            };

        self.messages.send_client(cid, ClientResponse::EntityAppear(eid, appearance, name));
    }

    fn on_entity_disappear(&mut self, cid: ClientId, eid: EntityId) {
        let time =
            if let Some(entity) = self.world.get_entity(eid) {
                entity.motion().start_time
            } else {
                0
            };
        // TODO: figure out if it's actually useful to send the time here.  The client currently
        // ignores it.
        self.messages.send_client(cid, ClientResponse::EntityGone(eid, time));
    }

    fn on_entity_update(&mut self, cid: ClientId, eid: EntityId) {
        let entity = self.world.entity(eid);

        let motion = entity.motion().clone();
        let anim = entity.anim();
        self.messages.send_client(cid, ClientResponse::EntityUpdate(eid, motion, anim));
    }
}
