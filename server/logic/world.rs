use physics::{CHUNK_SIZE, TILE_SIZE};

use types::*;
use util::SmallSet;

use chunks;
use engine::glue::*;
use world::{self, World};
use world::object::*;
use vision::{self, vision_region};


impl<'a, 'd> world::Hooks for WorldHooks<'a, 'd> {
    // No client_create callback because clients are added to vision in the logic::client code.

    fn on_client_destroy(&mut self, cid: ClientId) {
        self.script_mut().cb_client_destroyed(cid);
        // TODO: should this be here or in logic::clients?
        vision::Fragment::remove_client(&mut self.as_vision_fragment(), cid);
    }

    fn on_client_change_pawn(&mut self,
                             cid: ClientId,
                             _old_pawn: Option<EntityId>,
                             _new_pawn: Option<EntityId>) {
        let now = self.now();
        let center = match self.world().client(cid).pawn() {
            Some(e) => e.pos(now),
            None => scalar(0),
        };

        let region = vision_region(center);
        vision::Fragment::set_client_view(&mut self.as_vision_fragment(), cid, region);
    }


    fn on_terrain_chunk_create(&mut self, cpos: V2) {
        vision::Fragment::add_chunk(&mut self.as_vision_fragment(), cpos);
    }

    fn on_terrain_chunk_destroy(&mut self, cpos: V2) {
        vision::Fragment::remove_chunk(&mut self.as_vision_fragment(), cpos);
    }

    fn on_chunk_invalidate(&mut self, cpos: V2) {
        chunks::UpdateFragment::update(&mut self.as_chunks_update_fragment(), cpos);
        vision::Fragment::update_chunk(&mut self.as_vision_fragment(), cpos);
    }


    fn on_entity_create(&mut self, eid: EntityId) {
        let area = entity_area(self.world(), eid);
        vision::Fragment::add_entity(&mut self.as_vision_fragment(), eid, area);
    }

    fn on_entity_destroy(&mut self, eid: EntityId) {
        self.script_mut().cb_entity_destroyed(eid);
        vision::Fragment::remove_entity(&mut self.as_vision_fragment(), eid);
    }

    fn on_entity_motion_change(&mut self, eid: EntityId) {
        let area = entity_area(self.world(), eid);
        vision::Fragment::set_entity_area(&mut self.as_vision_fragment(), eid, area);
    }

    fn on_entity_appearance_change(&mut self, eid: EntityId) {
        vision::Fragment::update_entity_appearance(&mut self.as_vision_fragment(), eid);
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
        vision::Fragment::update_inventory(&mut self.as_vision_fragment(),
                                           iid, item_id, old_count, new_count);
    }
}

impl<'a, 'd> world::Hooks for HiddenWorldHooks<'a, 'd> {
    fn on_client_destroy(&mut self, cid: ClientId) {
        self.script_mut().cb_client_destroyed(cid);
    }

    // No terrain_chunk_destroy because ScriptEngine doesn't have such a callback

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
