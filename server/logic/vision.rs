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
use messages::{ClientResponse, Dialog};
use physics_;
use script;
use terrain_gen;
use world::{self, World};
use world::object::*;
use world::save::{self, ObjectReader, ObjectWriter, ReadHooks, WriteHooks};
use vision::{self, vision_region};


impl<'a, 'd> vision::Hooks for VisionHooks<'a, 'd> {
    fn on_chunk_update(&mut self, cid: ClientId, pos: V2) {
        use util::encode_rle16;
        let tc = unwrap_or!(self.chunks().get_terrain(pos),
            { warn!("no cached terrain available for {:?}", pos); return });
        let data = encode_rle16(tc.iter().map(|&x| x));
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


    fn on_inventory_appear(&mut self, cid: ClientId, iid: InventoryId) {
        let i = self.world().inventory(iid);

        let updates = i.contents().iter().map(|(&item, &count)| (item, 0, count)).collect();
        self.messages().send_client(cid, ClientResponse::InventoryUpdate(iid, updates));
    }

    fn on_inventory_update(&mut self,
                           cid: ClientId,
                           iid: InventoryId,
                           item_id: ItemId,
                           old_count: u8,
                           new_count: u8) {
        let update = vec![(item_id, old_count, new_count)];
        self.messages().send_client(cid, ClientResponse::InventoryUpdate(iid, update));
    }
}
