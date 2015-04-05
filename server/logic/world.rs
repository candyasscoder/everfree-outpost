use physics::{CHUNK_SIZE, TILE_SIZE};

use types::*;
use util::SmallSet;
use physics::Shape;

use cache::TerrainCache;
use chunks;
use data::StructureTemplate;
use engine::glue::*;
use engine::split::Open;
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

        let Open { world, cache, .. } = (**self).open();
        warn_on_err!(cache.add_chunk(world, cpos));
    }

    fn on_terrain_chunk_destroy(&mut self, cpos: V2) {
        vision::Fragment::remove_chunk(&mut self.as_vision_fragment(), cpos);

        self.cache_mut().remove_chunk(cpos);
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


    fn on_structure_create(&mut self, sid: StructureId) {
        new_structure(self, sid);
    }

    fn on_structure_destroy(&mut self, sid: StructureId, old_bounds: Region) {
        old_structure(self, sid, old_bounds);
        self.script_mut().cb_structure_destroyed(sid);
    }

    fn on_structure_replace(&mut self, sid: StructureId, old_bounds: Region) {
        old_structure(self, sid, old_bounds);
        new_structure(self, sid);
    }

    fn check_structure_placement(&self,
                                 template: &StructureTemplate,
                                 pos: V3) -> bool {
        check_structure_placement(self.world(), self.cache(), template, pos)
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

fn new_structure(wh: &mut WorldHooks,
                 sid: StructureId) {
    let area = structure_area(wh.world(), sid);
    vision::Fragment::add_structure(&mut wh.as_vision_fragment(), sid, area);

    let Open { world, cache, .. } = (**wh).open();
    let s = world.structure(sid);
    cache.update_region(world, s.bounds());
}

fn old_structure(wh: &mut WorldHooks,
                 sid: StructureId,
                 old_bounds: Region) {
    vision::Fragment::remove_structure(&mut wh.as_vision_fragment(), sid);

    let Open { world, cache, .. } = (**wh).open();
    cache.update_region(world, old_bounds);
}


// HiddenWorldHooks is like WorldHooks but does not send updates to clients.  Only the server's
// internal data structures are updated.
impl<'a, 'd> world::Hooks for HiddenWorldHooks<'a, 'd> {
    fn on_client_destroy(&mut self, cid: ClientId) {
        self.script_mut().cb_client_destroyed(cid);
    }

    fn on_terrain_chunk_create(&mut self, cpos: V2) {
        vision::Fragment::add_chunk(&mut self.as_hidden_vision_fragment(), cpos);

        let Open { world, cache, .. } = (**self).open();
        warn_on_err!(cache.add_chunk(world, cpos));
    }

    fn on_terrain_chunk_destroy(&mut self, cpos: V2) {
        vision::Fragment::remove_chunk(&mut self.as_hidden_vision_fragment(), cpos);

        self.cache_mut().remove_chunk(cpos);
    }

    fn on_entity_destroy(&mut self, eid: EntityId) {
        self.script_mut().cb_entity_destroyed(eid);
    }


    fn on_structure_create(&mut self, sid: StructureId) {
        new_structure_hidden(self, sid);
    }

    fn on_structure_destroy(&mut self, sid: StructureId, old_bounds: Region) {
        old_structure_hidden(self, sid, old_bounds);
        self.script_mut().cb_structure_destroyed(sid);
    }

    fn on_structure_replace(&mut self, sid: StructureId, old_bounds: Region) {
        old_structure_hidden(self, sid, old_bounds);
        new_structure_hidden(self, sid);
    }

    fn check_structure_placement(&self,
                                 template: &StructureTemplate,
                                 pos: V3) -> bool {
        check_structure_placement(self.world(), self.cache(), template, pos)
    }


    fn on_inventory_destroy(&mut self, iid: InventoryId) {
        self.script_mut().cb_inventory_destroyed(iid);
    }
}

fn new_structure_hidden(hwh: &mut HiddenWorldHooks,
                        sid: StructureId) {
    let area = structure_area(hwh.world(), sid);
    vision::Fragment::add_structure(&mut hwh.as_hidden_vision_fragment(), sid, area);

    let Open { world, cache, .. } = (**hwh).open();
    let s = world.structure(sid);
    cache.update_region(world, s.bounds());
}

fn old_structure_hidden(hwh: &mut HiddenWorldHooks,
                        sid: StructureId,
                        old_bounds: Region) {
    vision::Fragment::remove_structure(&mut hwh.as_hidden_vision_fragment(), sid);


    let Open { world, cache, .. } = (**hwh).open();
    cache.update_region(world, old_bounds);
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

pub fn structure_area(w: &World, sid: StructureId) -> SmallSet<V2> {
    let s = w.structure(sid);

    let mut area = SmallSet::new();
    for p in s.bounds().reduce().div_round_signed(CHUNK_SIZE).points() {
        area.insert(p);
    }

    area
}

fn check_structure_placement(world: &World,
                             cache: &TerrainCache,
                             template: &StructureTemplate,
                             pos: V3) -> bool {
    let data = world.data();
    let bounds = Region::new(scalar(0), template.size) + pos;

    for p in bounds.points() {
        let cpos = p.reduce().div_floor(scalar(CHUNK_SIZE));

        let tc = unwrap_or!(world.get_terrain_chunk(cpos), return false);
        let shape = data.block_data.shape(tc.block(tc.bounds().index(p)));
        match shape {
            Shape::Empty => {},
            Shape::Floor if p.z == pos.z => {},
            _ => {
                info!("placement failed due to terrain");
                return false;
            },
        }

        let entry = unwrap_or!(cache.get(cpos), return false);
        let mask = entry.layer_mask[tc.bounds().index(p)];
        if mask & (1 << template.layer as usize) != 0 {
            info!("placement failed due to layering");
            return false;
        }
    }

    true
}
