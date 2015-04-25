use physics::{CHUNK_SIZE, TILE_SIZE};

use types::*;
use util::SmallSet;
use physics::Shape;

use cache::TerrainCache;
use chunks;
use data::StructureTemplate;
use engine::glue::*;
use engine::split::Open;
use world::{self, World, Entity, Structure};
use world::object::*;
use vision::{self, vision_region};


macro_rules! impl_world_Hooks {
    ($WorldHooks:ident, $as_vision_fragment:ident,
     $old_structure:ident, $new_structure:ident) => {

impl<'a, 'd> world::Hooks for $WorldHooks<'a, 'd> {
    // We should never get client callbacks in the HiddenWorldHooks variant.

    // No client_create callback because clients are added to vision in the logic::client code.

    fn on_client_destroy(&mut self, cid: ClientId) {
        self.script_mut().cb_client_destroyed(cid);
        // TODO: should this be here or in logic::clients?
        vision::Fragment::remove_client(&mut self.$as_vision_fragment(), cid);
    }

    fn on_client_change_pawn(&mut self,
                             cid: ClientId,
                             _old_pawn: Option<EntityId>,
                             _new_pawn: Option<EntityId>) {
        let now = self.now();
        let (plane, center) = match self.world().client(cid).pawn() {
            Some(e) => (e.plane_id(), e.pos(now)),
            None => (PLANE_LIMBO, scalar(0)),
        };

        let region = vision_region(center);
        vision::Fragment::set_client_view(&mut self.$as_vision_fragment(), cid, plane, region);
    }


    fn on_terrain_chunk_create(&mut self, tcid: TerrainChunkId) {
        let (pid, cpos) = {
            let tc = self.world().terrain_chunk(tcid);
            (tc.plane_id(), tc.chunk_pos())
        };
        vision::Fragment::add_terrain_chunk(&mut self.$as_vision_fragment(), tcid, pid, cpos);

        let Open { world, cache, .. } = (**self).open();
        warn_on_err!(cache.add_chunk(world, pid, cpos));
    }

    fn on_terrain_chunk_destroy(&mut self, tcid: TerrainChunkId, pid: PlaneId, cpos: V2) {
        vision::Fragment::remove_terrain_chunk(&mut self.$as_vision_fragment(), tcid);

        self.cache_mut().remove_chunk(pid, cpos);
    }


    fn on_entity_create(&mut self, eid: EntityId) {
        let (plane, area) = {
            let e = self.world().entity(eid);
            (e.plane_id(), entity_area(self.world().entity(eid)))
        };
        trace!("entity {:?} created at {:?}", eid, plane);
        vision::Fragment::add_entity(&mut self.$as_vision_fragment(), eid, plane, area);
    }

    fn on_entity_destroy(&mut self, eid: EntityId) {
        self.script_mut().cb_entity_destroyed(eid);
        vision::Fragment::remove_entity(&mut self.$as_vision_fragment(), eid);
    }

    fn on_entity_motion_change(&mut self, eid: EntityId) {
        let (plane, area) = {
            let e = self.world().entity(eid);
            (e.plane_id(), entity_area(self.world().entity(eid)))
        };
        trace!("entity {:?} motion changed to {:?}", eid, plane);
        vision::Fragment::set_entity_area(&mut self.$as_vision_fragment(), eid, plane, area);
    }

    fn on_entity_appearance_change(&mut self, eid: EntityId) {
        vision::Fragment::update_entity_appearance(&mut self.$as_vision_fragment(), eid);
    }

    fn on_entity_plane_change(&mut self, eid: EntityId) {
        trace!("entity {:?} plane changed", eid);
        self.on_entity_motion_change(eid);
    }


    fn on_structure_create(&mut self, sid: StructureId) {
        $new_structure(self, sid);
    }

    fn on_structure_destroy(&mut self,
                            sid: StructureId,
                            pid: PlaneId,
                            old_bounds: Region) {
        $old_structure(self, sid, pid, old_bounds);
        self.script_mut().cb_structure_destroyed(sid);
    }

    fn on_structure_replace(&mut self,
                            sid: StructureId,
                            pid: PlaneId,
                            old_bounds: Region) {
        $old_structure(self, sid, pid, old_bounds);
        $new_structure(self, sid);
    }

    fn check_structure_placement(&self,
                                 template: &StructureTemplate,
                                 pid: PlaneId,
                                 pos: V3) -> bool {
        check_structure_placement(self.world(), self.cache(), template, pid, pos)
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
        vision::Fragment::update_inventory(&mut self.$as_vision_fragment(),
                                           iid, item_id, old_count, new_count);
    }
}

fn $new_structure(wh: &mut $WorldHooks,
                  sid: StructureId) {
    let (pid, area) = {
        let s = wh.world().structure(sid);
        (s.plane_id(), structure_area(s))
    };
    vision::Fragment::add_structure(&mut wh.$as_vision_fragment(), sid, pid, area);

    let Open { world, cache, .. } = (**wh).open();
    let s = world.structure(sid);
    cache.update_region(world, pid, s.bounds());
}

fn $old_structure(wh: &mut $WorldHooks,
                  sid: StructureId,
                  old_pid: PlaneId,
                  old_bounds: Region) {
    vision::Fragment::remove_structure(&mut wh.$as_vision_fragment(), sid);

    let Open { world, cache, .. } = (**wh).open();
    cache.update_region(world, old_pid, old_bounds);
}

// End of macro_rules
    };
}


impl_world_Hooks!(WorldHooks, as_vision_fragment,
                  old_structure, new_structure);
impl_world_Hooks!(HiddenWorldHooks, as_hidden_vision_fragment,
                  old_structure_hidden, new_structure_hidden);


pub fn entity_area(e: ObjectRef<Entity>) -> SmallSet<V2> {
    let mut area = SmallSet::new();

    let a = e.motion().start_pos.reduce().div_floor(scalar(CHUNK_SIZE * TILE_SIZE));
    let b = e.motion().end_pos.reduce().div_floor(scalar(CHUNK_SIZE * TILE_SIZE));

    area.insert(a);
    area.insert(b);
    area
}

pub fn structure_area(s: ObjectRef<Structure>) -> SmallSet<V2> {
    let mut area = SmallSet::new();
    for p in s.bounds().reduce().div_round_signed(CHUNK_SIZE).points() {
        area.insert(p);
    }

    area
}


// There are (currently) three layers for structure placement, each with distinct properties.
//
// Layer 0: Floor-type structures.  House floor, road, etc.  These can be placed over terrain
// floors and in empty space.
//
// Layer 1: Solid structures.  House wall, anvil, chest, etc.  These require empty space throughout
// their volume and also a floor at the bottom.
//
// Layer 2: Solid attachments.  Cabinets, bookshelves, etc.  These can be placed like Layer 1
// structures (floor + empty space above), or they can instead be placed over a Layer 1 structure
// with no shape restrictions.  In the case of placement over an existing Layer 1 structure, the
// script doing the placement is responsible for enforcing any additional invariants.

const PLACEMENT_MASK: [u8; 3] = [
    0x1,    // Layer 0 can be placed under existing structures.
    0x6,    // Layer 1 can be placed over Layer 0, but not under Layer 2.
    0x4,    // Layer 2 can be placed over Layer 0 and 1.
];

fn check_structure_placement(world: &World,
                             cache: &TerrainCache,
                             template: &StructureTemplate,
                             pid: PlaneId,
                             base_pos: V3) -> bool {
    let data = world.data();
    let bounds = Region::new(scalar(0), template.size) + base_pos;

    let p = unwrap_or!(world.get_plane(pid), return false);
    for pos in bounds.points() {
        let cpos = pos.reduce().div_floor(scalar(CHUNK_SIZE));

        let tc = unwrap_or!(p.get_terrain_chunk(cpos), return false);
        let shape = data.block_data.shape(tc.block(tc.bounds().index(pos)));

        let entry = unwrap_or!(cache.get(pid, cpos), return false);
        let mask = entry.layer_mask[tc.bounds().index(pos)];

        let shape_ok = match template.layer {
            0 => check_shape_0(shape, pos.z == base_pos.z),
            1 => check_shape_1(shape, pos.z == base_pos.z),
            2 => {
                if mask & (1 << 1) != 0 {
                    // Allow unrestricted placement over layer-1 structures.
                    true
                } else {
                    check_shape_1(shape, pos.z == base_pos.z)
                }
            },
            x => {
                info!("unexpected template layer: {}", x);
                false
            },
        };

        if !shape_ok {
            info!("placement failed due to terrain");
            return false;
        }

        if mask & PLACEMENT_MASK[template.layer as usize] != 0 {
            info!("placement failed due to layering");
            return false;
        }
    }

    true
}

fn check_shape_0(shape: Shape, is_bottom: bool) -> bool {
    match shape {
        Shape::Empty => true,
        Shape::Floor if is_bottom => true,
        _ => false,
    }
}

fn check_shape_1(shape: Shape, is_bottom: bool) -> bool {
    match shape {
        Shape::Empty if !is_bottom => true,
        Shape::Floor if is_bottom => true,
        _ => false,
    }
}
