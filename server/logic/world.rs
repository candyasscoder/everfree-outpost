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


impl<'a, 'd> world::Hooks for WorldHooks<'a, 'd> {
    fn on_client_create(&mut self, cid: ClientId) {
    }

    fn on_client_destroy(&mut self, cid: ClientId) {
        self.script_mut().cb_client_destroyed(cid);
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
        info!("created {:?}", pos);
        let (mut h, mut e): (VisionHooks, _) = self.borrow().split_off();
        e.vision_mut().add_chunk(pos, &mut h);
    }

    fn on_terrain_chunk_destroy(&mut self, pos: V2) {
        let (mut h, mut e): (VisionHooks, _) = self.borrow().split_off();
        e.vision_mut().remove_chunk(pos, &mut h);
    }

    fn on_chunk_invalidate(&mut self, pos: V2) {
        info!("invalidated {:?}", pos);
        let (mut h, mut e): (VisionHooks, _) = self.borrow().split_off();
        e.vision_mut().update_chunk(pos, &mut h);
    }


    fn on_entity_create(&mut self, eid: EntityId) {
        let area = entity_area(self.world(), eid);
        let (mut h, mut e): (VisionHooks, _) = self.borrow().split_off();
        e.vision_mut().add_entity(eid, area, &mut h);
    }

    fn on_entity_destroy(&mut self, eid: EntityId) {
        self.script_mut().cb_entity_destroyed(eid);
        let (mut h, mut e): (VisionHooks, _) = self.borrow().split_off();
        e.vision_mut().remove_entity(eid, &mut h);
    }

    fn on_entity_motion_change(&mut self, eid: EntityId) {
        let area = entity_area(self.world(), eid);
        let (mut h, mut e): (VisionHooks, _) = self.borrow().split_off();
        e.vision_mut().set_entity_area(eid, area, &mut h);
    }


    fn on_structure_destroy(&mut self, sid: StructureId) {
        self.script_mut().cb_structure_destroyed(sid);
    }


    // No lifecycle callbacks for inventories, because Vision doesn't care what inventories exist,
    // only what inventories are actually subscribed to.

    fn on_inventory_destroy(&mut self, iid: InventoryId) {
        self.script_mut().cb_inventory_destroyed(iid);
    }

    fn on_inventory_update(&mut self,
                           iid: InventoryId,
                           item_id: ItemId,
                           old_count: u8,
                           new_count: u8) {
        let (mut h, mut e): (VisionHooks, _) = self.borrow().split_off();
        e.vision_mut().update_inventory(iid, item_id, old_count, new_count, &mut h);
    }
}

impl<'a, 'd> world::Hooks for HiddenWorldHooks<'a, 'd> {
    fn on_client_destroy(&mut self, cid: ClientId) {
        self.script_mut().cb_client_destroyed(cid);
    }

    fn on_terrain_chunk_destroy(&mut self, pos: V2) {
        // ScriptEngine doesn't have a callback for this one
    }

    fn on_entity_destroy(&mut self, eid: EntityId) {
        self.script_mut().cb_entity_destroyed(eid);
    }

    fn on_structure_destroy(&mut self, sid: StructureId) {
        self.script_mut().cb_structure_destroyed(sid);
    }

    fn on_inventory_destroy(&mut self, iid: InventoryId) {
        self.script_mut().cb_inventory_destroyed(iid);
    }
}

pub fn entity_area(w: &World, eid: EntityId) -> SmallSet<V2> {
    let e = w.entity(eid);
    let mut area = SmallSet::new();

    let a = e.motion().start_pos.reduce().div_floor(scalar(CHUNK_SIZE * TILE_SIZE));
    let b = e.motion().end_pos.reduce().div_floor(scalar(CHUNK_SIZE * TILE_SIZE));

    area.insert(a);
    area.insert(b);
    area
}
