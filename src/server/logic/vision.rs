use std::borrow::ToOwned;

use types::*;

use engine::glue::*;
use messages::ClientResponse;
use world;
use world::object::*;
use vision;


impl<'a, 'd> vision::Hooks for VisionHooks<'a, 'd> {
    fn on_terrain_chunk_appear(&mut self,
                               cid: ClientId,
                               tcid: TerrainChunkId,
                               cpos: V2) {
        self.on_terrain_chunk_update(cid, tcid, cpos);
    }

    fn on_terrain_chunk_update(&mut self,
                               cid: ClientId,
                               tcid: TerrainChunkId,
                               cpos: V2) {
        use util::encode_rle16;
        let tc = unwrap_or!(self.world().get_terrain_chunk(tcid),
            { warn!("no terrain available for {:?}", tcid); return });
        let data = encode_rle16(tc.blocks().iter().map(|&x| x));
        self.messages().send_client(cid, ClientResponse::TerrainChunk(cpos, data));
    }


    fn on_entity_appear(&mut self, cid: ClientId, eid: EntityId) {
        trace!("on_entity_appear({:?}, {:?})", cid, eid);
        {
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

        self.on_entity_motion_update(cid, eid);
    }

    fn on_entity_disappear(&mut self, cid: ClientId, eid: EntityId) {
        trace!("on_entity_disappear({:?}, {:?})", cid, eid);
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

    fn on_entity_motion_update(&mut self, cid: ClientId, eid: EntityId) {
        trace!("on_entity_motion_update({:?}, {:?})", cid, eid);
        let entity = self.world().entity(eid);

        let motion = entity.motion().clone();
        let anim = entity.anim();
        self.messages().send_client(cid, ClientResponse::EntityUpdate(eid, motion, anim));
    }

    fn on_entity_appearance_update(&mut self, cid: ClientId, eid: EntityId) {
        trace!("on_entity_appearance_update({:?}, {:?})", cid, eid);
        self.on_entity_appear(cid, eid);
    }


    fn on_plane_change(&mut self,
                       cid: ClientId,
                       _: PlaneId,
                       pid: PlaneId) {
        // TODO: super hack.  add a flags field to the plane or something.
        let is_dark = match self.world().get_plane(pid) {
            Some(p) => p.name() != "Everfree Forest",
            None => true,
        };
        self.messages().send_client(cid, ClientResponse::PlaneFlags(is_dark as u32));
    }


    fn on_structure_appear(&mut self, cid: ClientId, sid: StructureId) {
        let s = self.world().structure(sid);
        self.messages().send_client(cid, ClientResponse::StructureAppear(
                sid, s.template_id(), s.pos()));
    }

    fn on_structure_disappear(&mut self, cid: ClientId, sid: StructureId) {
        self.messages().send_client(cid, ClientResponse::StructureGone(sid));
    }

    fn on_structure_template_change(&mut self, cid: ClientId, sid: StructureId) {
        let s = self.world().structure(sid);
        self.messages().send_client(cid, ClientResponse::StructureReplace(sid, s.template_id()));
    }


    fn on_inventory_appear(&mut self, cid: ClientId, iid: InventoryId) {
        let i = self.world().inventory(iid);
        let contents = i.contents().iter().map(|&x| x).collect();
        self.messages().send_client(
            cid, ClientResponse::InventoryAppear(iid, contents));
    }

    fn on_inventory_disappear(&mut self, cid: ClientId, iid: InventoryId) {
        self.messages().send_client(
            cid, ClientResponse::InventoryGone(iid));
    }

    fn on_inventory_update(&mut self,
                           cid: ClientId,
                           iid: InventoryId,
                           slot_idx: u8) {
        let i = self.world().inventory(iid);
        let item = i.contents()[slot_idx as usize];
        self.messages().send_client(
            cid, ClientResponse::InventoryUpdate(iid, slot_idx, item));
    }
}
