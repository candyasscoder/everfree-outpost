use physics::{self, Shape, ShapeSource};
use physics::{CHUNK_SIZE, CHUNK_BITS, CHUNK_MASK, TILE_SIZE};

use types::*;
use util::StrResult;

use cache::TerrainCache;
use data::Data;
use world::{self, World};
use world::Motion;
use world::object::*;


pub struct Physics<'d> {
    data: &'d Data,
}

impl<'d> Physics<'d> {
    pub fn new(data: &'d Data) -> Physics<'d> {
        Physics {
            data: data,
        }
    }
}


struct ChunksSource<'a> {
    cache: &'a TerrainCache,
    base_tile: V3,
    plane: PlaneId,
}

impl<'a> ShapeSource for ChunksSource<'a> {
    fn get_shape(&self, pos: V3) -> Shape {
        if pos.z < 0 || pos.z >= CHUNK_SIZE {
            return Shape::Empty;
        }

        let pos = pos + self.base_tile;

        let offset = pos & scalar(CHUNK_MASK);
        let cpos = (pos >> CHUNK_BITS).reduce();

        if let Some(entry) = self.cache.get(self.plane, cpos) {
            let idx = Region::new(scalar(0), scalar(CHUNK_SIZE)).index(offset);
            entry.shape[idx]
        } else {
            return Shape::Empty;
        }
    }
}


pub trait Fragment<'d> {
    fn with_cache<F, R>(&mut self, f: F) -> R
        where F: FnOnce(&mut Physics<'d>, &TerrainCache, &World<'d>) -> R;

    type WF: world::Fragment<'d>;
    fn with_world<F, R>(&mut self, f: F) -> R
        where F: FnOnce(&mut Self::WF) -> R;

    fn set_velocity(&mut self, now: Time, eid: EntityId, target: V3) -> StrResult<()> {
        use world::Fragment;
        try!(self.with_world(|wf| -> StrResult<()> {
            let mut e = unwrap!(wf.get_entity_mut(eid));
            e.set_target_velocity(target);
            Ok(())
        }));
        self.update(now, eid)
    }

    fn update(&mut self, now: Time, eid: EntityId) -> StrResult<()> {
        use world::Fragment;

        let motion = try!(self.with_cache(|_sys, cache, world| -> StrResult<_> {
            let e = unwrap!(world.get_entity(eid));

            // Run the physics calculation

            // TODO: hardcoded constant based on entity size
            let start_pos = e.pos(now);
            let velocity = e.target_velocity();
            let size = V3::new(32, 32, 64);

            let chunk_px = CHUNK_SIZE * TILE_SIZE;
            let base_chunk = start_pos.div_floor(scalar(chunk_px)) - scalar::<V2>(3).extend(0);
            let base_tile = base_chunk * scalar(CHUNK_SIZE);
            let base_px = base_tile * scalar(TILE_SIZE);

            let source = ChunksSource {
                cache: cache,
                base_tile: base_tile,
                plane: e.plane_id(),
            };
            let (mut end_pos, mut dur) =
                physics::collide(&source, start_pos - base_px, size, velocity);
            end_pos = end_pos + base_px;

            // NB: keep this in sync with client/js/physics.js  computeForecast
            if dur > DURATION_MAX as i32 {
                let offset = end_pos - start_pos;
                end_pos = start_pos + offset * scalar(DURATION_MAX as i32) / scalar(dur);
                dur = DURATION_MAX as i32;
            } else if dur == 0 {
                dur = DURATION_MAX as i32;
            }

            Ok(Motion {
                start_time: now,
                duration: dur as Duration,
                start_pos: start_pos,
                end_pos: end_pos,
            })
        }));

        self.with_world(|wf| {
            let data = wf.world().data();
            let mut e = wf.entity_mut(eid);

            // Compute extra information for the entity.
            let velocity = e.target_velocity();
            let dir = velocity.signum();
            // TODO: player speed handling shouldn't be here
            let speed = velocity.abs().max() / 50;

            let facing = 
                if dir != scalar(0) {
                    dir
                } else {
                    e.facing()
                };

            const ANIM_DIR_COUNT: AnimId = 8;
            static SPEED_NAME_MAP: [&'static str; 4] = ["stand", "walk", "", "run"];
            let idx = (3 * (facing.x + 1) + (facing.y + 1)) as usize;
            let anim_dir = [5, 4, 3, 6, 0, 2, 7, 0, 1][idx];
            let anim_name = format!("pony/{}-{}",
                                    SPEED_NAME_MAP[speed as usize],
                                    anim_dir);
            let anim = data.animations.get_id(&anim_name);

            e.set_anim(anim);
            e.set_facing(facing);
            e.set_motion(motion);
        });
        Ok(())
    }
}
