use std::iter;
use libphysics::{CHUNK_SIZE, TILE_SIZE};

use types::*;
use util::SmallSet;
use util::StrResult;

use data::StructureTemplate;
use engine::glue::*;
use engine::split::{Open, EngineRef};
use logic;
use messages::{ClientResponse, SyncKind};
use physics;
use world::{self, World, Entity, Structure};
use world::object::*;
use vision::{self, vision_region};


macro_rules! impl_world_Hooks {
    ($WorldHooks:ident, $as_vision_fragment:ident) => {

impl<'a, 'd> world::Hooks for $WorldHooks<'a, 'd> {
    // We should never get client callbacks in the HiddenWorldHooks variant.

    // No client_create callback because clients are added to vision in the logic::client code.

    fn on_client_destroy(&mut self, cid: ClientId) {
        self.script_mut().cb_client_destroyed(cid);
        // TODO: should this be here or in logic::clients?
        vision::Fragment::remove_client(&mut self.$as_vision_fragment(), cid);
    }

    fn on_client_change_pawn(&mut self,
                             _cid: ClientId,
                             _old_pawn: Option<EntityId>,
                             new_pawn: Option<EntityId>) {
        if let Some(eid) = new_pawn {
            // TODO: handle this properly.  needs to send a fresh Init message to the client
            self.schedule_view_update(eid);
        }
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

    fn on_terrain_chunk_update(&mut self, tcid: TerrainChunkId) {
        // TODO: need a system to avoid resending the entire chunk every time.
        let (pid, bounds) = {
            let tc = self.world().terrain_chunk(tcid);
            (tc.plane_id(), tc.bounds())
        };
        vision::Fragment::update_terrain_chunk(&mut self.$as_vision_fragment(), tcid);

        let Open { world, cache, .. } = (**self).open();
        cache.update_region(world, pid, bounds);
    }


    fn on_entity_create(&mut self, eid: EntityId) {
        let (plane, area, end_time) = {
            let e = self.world().entity(eid);
            (e.plane_id(),
             entity_area(self.world().entity(eid)),
             e.motion().end_time())
        };
        trace!("entity {:?} created at {:?}", eid, plane);
        // TODO: use a default plane/area for add_entity, then just call on_motion_change
        vision::Fragment::add_entity(&mut self.$as_vision_fragment(), eid, plane, area);
        self.schedule_physics_update(eid, end_time);
        // Might have an owner pre-set, if it's been loaded instead of newly created.
        self.schedule_view_update(eid);
    }

    fn on_entity_destroy(&mut self, eid: EntityId) {
        self.script_mut().cb_entity_destroyed(eid);
        vision::Fragment::remove_entity(&mut self.$as_vision_fragment(), eid);
    }

    fn on_entity_motion_change(&mut self, eid: EntityId) {
        let (plane, area, end_time) = {
            let e = self.world().entity(eid);
            (e.plane_id(),
             entity_area(self.world().entity(eid)),
             e.motion().end_time())
        };
        trace!("entity {:?} motion changed to {:?}", eid, plane);
        vision::Fragment::set_entity_area(&mut self.$as_vision_fragment(), eid, plane, area);
        self.schedule_physics_update(eid, end_time);
        self.schedule_view_update(eid);
    }

    fn on_entity_appearance_change(&mut self, eid: EntityId) {
        vision::Fragment::update_entity_appearance(&mut self.$as_vision_fragment(), eid);
    }

    fn on_entity_plane_change(&mut self, eid: EntityId) {
        trace!("entity {:?} plane changed", eid);
        self.on_entity_motion_change(eid);
    }


    fn on_structure_create(&mut self, sid: StructureId) {
        let (pid, area) = {
            let s = self.world().structure(sid);
            (s.plane_id(), structure_area(s))
        };
        vision::Fragment::add_structure(&mut self.$as_vision_fragment(), sid, pid, area);

        let Open { world, cache, .. } = (**self).open();
        let s = world.structure(sid);
        cache.update_region(world, pid, s.bounds());
    }

    fn on_structure_destroy(&mut self,
                            sid: StructureId,
                            old_pid: PlaneId,
                            old_bounds: Region) {
        vision::Fragment::remove_structure(&mut self.$as_vision_fragment(), sid);

        {
            let Open { world, cache, .. } = (**self).open();
            cache.update_region(world, old_pid, old_bounds);
        }

        self.script_mut().cb_structure_destroyed(sid);
    }

    fn on_structure_replace(&mut self,
                            sid: StructureId,
                            pid: PlaneId,
                            old_bounds: Region) {
        {
            let Open { world, cache, .. } = (**self).open();
            cache.update_region(world, pid, old_bounds);
        }

        let (pid, area) = {
            let s = self.world().structure(sid);
            (s.plane_id(), structure_area(s))
        };
        vision::Fragment::set_structure_area(&mut self.$as_vision_fragment(), sid, pid, area);
        vision::Fragment::change_structure_template(&mut self.$as_vision_fragment(), sid);

        let Open { world, cache, .. } = (**self).open();
        let s = world.structure(sid);
        cache.update_region(world, pid, old_bounds.join(s.bounds()));
    }

    fn check_structure_placement(&self,
                                 template: &StructureTemplate,
                                 pid: PlaneId,
                                 pos: V3) -> bool {
        let cache = self.cache();
        let chunk_bounds = Region::new(scalar(0), scalar(CHUNK_SIZE));
        check_structure_placement(self.world(), template, pid, pos, |pos| {
            let cpos = pos.reduce().div_floor(scalar(CHUNK_SIZE));
            let entry = unwrap_or!(cache.get(pid, cpos), return None);
            let cur_chunk_bounds = chunk_bounds + cpos.extend(0) * scalar(CHUNK_SIZE);
            let mask = entry.layer_mask[cur_chunk_bounds.index(pos)];
            Some(mask)
        })
    }

    fn check_structure_replacement(&self,
                                   sid: StructureId,
                                   new_template: &StructureTemplate,
                                   pid: PlaneId,
                                   pos: V3) -> bool {
        let bounds = Region::new(pos, pos + new_template.size);
        let mask = unwrap_or!(compute_layer_mask_excluding(self.world(), pid, bounds, sid).ok(),
                              return false);
        check_structure_placement(self.world(), new_template, pid, pos, |pos| {
            if !bounds.contains(pos) {
                return None;
            }
            Some(mask[bounds.index(pos)])
        })
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

impl<'a, 'd> $WorldHooks<'a, 'd> {
    fn schedule_physics_update(&mut self, eid: EntityId, when: Time) {
        if let Some(cookie) = self.extra_mut().entity_physics_update_timer.remove(&eid) {
            self.timer_mut().cancel(cookie);
        }
        let cookie = self.timer_mut().schedule(when, move |eng| update_physics(eng, eid));
        self.extra_mut().entity_physics_update_timer.insert(eid, cookie);
    }

    pub fn schedule_view_update(&mut self, eid: EntityId) {
        let now = self.now();
        let cid;
        let when = {
            let e = unwrap_or!(self.world().get_entity(eid));
            let c = unwrap_or!(e.pawn_owner());
            cid = c.id();

            // If the client is not registered with the vision system, do nothing.
            let old_area = unwrap_or!(self.vision().client_view_area(c.id()));
            let new_area = vision_region(e.pos(now));

            if old_area != new_area {
                // Simple case: If the vision area needs to change immediately, schedule the update
                // to happen as soon as possible.
                Some(now)
            } else {
                // Complex case: Figure out when the pawn will move into a new chunk, and schedule
                // the update to happen at that time.
                let m = e.motion();
                let start = m.pos(now);
                let delta = m.end_pos - start;
                let dur = (m.end_time() - now) as i32;
                const CHUNK_PX: i32 = CHUNK_SIZE * TILE_SIZE;

                let hit_time = start.zip(delta, |x, dx| {
                    use std::i32;
                    if dx == 0 {
                        return i32::MAX;
                    }
                    let target = (x & !(CHUNK_PX - 1)) + if dx < 0 { -1 } else { CHUNK_PX };
                    // i32 math is okay here because `dir` maxes out at 2^16 and `target` at 2^9.
                    (dur * (target - x) + dx - 1) / dx
                }).min();
                if hit_time > dur {
                    // Won't hit a chunk boundary before the motion ends.
                    None
                } else {
                    Some(now + hit_time as Time)
                }
            }
        };

        if let Some(cookie) = self.extra_mut().client_view_update_timer.remove(&cid) {
            self.timer_mut().cancel(cookie);
        }
        if let Some(when) = when {
            let cookie = self.timer_mut().schedule(when, move |eng| {
                logic::client::update_view(eng, cid);
            });
            self.extra_mut().client_view_update_timer.insert(cid, cookie);
        }
    }
}

// End of macro_rules
    };
}


impl_world_Hooks!(WorldHooks, as_vision_fragment);
impl_world_Hooks!(HiddenWorldHooks, as_hidden_vision_fragment);


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

fn check_structure_placement<F>(world: &World,
                                template: &StructureTemplate,
                                pid: PlaneId,
                                base_pos: V3,
                                mut get_mask: F) -> bool
        where F: FnMut(V3) -> Option<u8> {
    let data = world.data();
    let bounds = Region::new(scalar(0), template.size) + base_pos;

    let p = unwrap_or!(world.get_plane(pid), return false);
    for pos in bounds.points() {
        let cpos = pos.reduce().div_floor(scalar(CHUNK_SIZE));

        let tc = unwrap_or!(p.get_terrain_chunk(cpos), return false);
        let shape = data.block_data.shape(tc.block(tc.bounds().index(pos)));

        let mask = unwrap_or!(get_mask(pos), return false);

        let shape_ok = match template.layer {
            0 => check_shape_0(shape, pos.z == base_pos.z, mask),
            1 => check_shape_1(shape, pos.z == base_pos.z, mask),
            2 => check_shape_2(shape, pos.z == base_pos.z, mask),
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

fn check_shape_0(shape: Shape, is_bottom: bool, _mask: u8) -> bool {
    if is_bottom {
        shape == Shape::Floor || shape == Shape::Empty
    } else {
        shape == Shape::Empty
    }
}

fn check_shape_1(shape: Shape, is_bottom: bool, mask: u8) -> bool {
    if is_bottom {
        mask & (1 << 0) != 0 || shape == Shape::Floor
    } else {
        mask & (1 << 0) == 0 && shape == Shape::Empty
    }
}

fn check_shape_2(shape: Shape, is_bottom: bool, mask: u8) -> bool {
    if mask & (1 << 1) != 0 {
        true
    } else {
        check_shape_1(shape, is_bottom, mask)
    }
}


fn compute_layer_mask_excluding(w: &World,
                                pid: PlaneId,
                                bounds: Region,
                                exclude_sid: StructureId) -> StrResult<Vec<u8>> {
    let mut result = iter::repeat(0_u8).take(bounds.volume() as usize).collect::<Vec<_>>();

    for cpos in bounds.reduce().div_round_signed(CHUNK_SIZE).points() {
        for s in w.chunk_structures(pid, cpos) {
            if s.id() == exclude_sid {
                continue;
            }

            for p in s.bounds().intersect(bounds).points() {
                let template = s.template();
                result[bounds.index(p)] |= 1 << (template.layer as usize);
            }
        }
    }

    Ok(result)
}


fn update_physics(mut eng: EngineRef, eid: EntityId) {
    let now = eng.now();
    warn_on_err!(physics::Fragment::update(&mut eng.as_physics_fragment(), now, eid));
    // When `update` changes the entity's motion, the hook will schedule the next update,
    // cancelling any currently pending update.
}

fn teleport_entity_internal(mut wf: WorldFragment,
                            eid: EntityId,
                            pid: Option<PlaneId>,
                            stable_pid: Option<Stable<PlaneId>>,
                            pos: V3) -> StrResult<()> {
    use world::Fragment;
    let now = wf.now();

    {
        let e = unwrap!(wf.world().get_entity(eid));
        let cid = e.pawn_owner().map(|c| c.id());
        if let Some(cid) = cid {
            // Only send desync message for long-range teleports.
            let dist = (e.pos(now) - pos).reduce().abs().max();
            let change_plane = (pid.is_some() && pid.unwrap() != e.plane_id()) ||
                (stable_pid.is_some() && stable_pid.unwrap() != e.stable_plane_id());

            // NB: Teleporting to another point within the current chunk will not cause a view
            // update to be scheduled, so there will never be a resync message.  That's why we set
            // the limit to CHUNK_SIZE * TILE_SIZE: traveling that distance along either the X or Y
            // axis will definitely move the entity into a different chunk.
            if dist >= CHUNK_SIZE * TILE_SIZE || change_plane {
                wf.messages().send_client(cid, ClientResponse::SyncStatus(SyncKind::Loading));
            }
        }
    }

    let mut e = wf.entity_mut(eid);
    if let Some(stable_pid) = stable_pid {
        try!(e.set_stable_plane_id(stable_pid));
    } else if let Some(pid) = pid {
        try!(e.set_plane_id(pid));
    }
    e.set_motion(world::Motion::stationary(pos, now));
    Ok(())
}

pub fn teleport_entity(wf: WorldFragment,
                       eid: EntityId,
                       pos: V3) -> StrResult<()> {
    teleport_entity_internal(wf, eid, None, None, pos)
}

pub fn teleport_entity_plane(wf: WorldFragment,
                             eid: EntityId,
                             pid: PlaneId,
                             pos: V3) -> StrResult<()> {
    teleport_entity_internal(wf, eid, Some(pid), None, pos)
}

pub fn teleport_entity_stable_plane(wf: WorldFragment,
                                    eid: EntityId,
                                    stable_pid: Stable<PlaneId>,
                                    pos: V3) -> StrResult<()> {
    teleport_entity_internal(wf, eid, None, Some(stable_pid), pos)
}
