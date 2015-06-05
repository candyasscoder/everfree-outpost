use physics::CHUNK_SIZE;

use types::*;

use world::{self, Hooks};
use world::object::*;


pub fn set_block_interior<'d, F>(wf: &mut F,
                                 pid: PlaneId,
                                 center: V3,
                                 base: &str,
                                 inside: bool) -> world::OpResult<()>
        where F: world::Fragment<'d> {
    let prefix = format!("{}/", base);

    let mut updates = [None; 9];
    let update_region = Region::new(center - V3::new(1, 1, 0),
                                    center + V3::new(2, 2, 1));

    debug!("set_block_interior: {:?}, {:?}, {}, {}", pid, center, base, inside);

    {
        #[derive(Clone, Copy)]
        enum Status {
            Uninitialized,
            Inside,
            Outside,
        }

        let mut cache = [Status::Uninitialized; 25];
        cache[2 * 5 + 2] = if inside { Status::Inside } else { Status::Outside };

        let w = wf.world();
        let bd = &w.data().block_data;
        let p = unwrap!(w.get_plane(pid));

        let cache_region = Region::new(center - V3::new(2, 2, 0),
                                       center + V3::new(3, 3, 1));
        for cpos in cache_region.reduce().div_round_signed(CHUNK_SIZE).points() {
            // Check that the chunk is loaded.
            let _ = unwrap!(p.get_terrain_chunk(cpos));
        }

        let mut is_inside = |pos| {
            let idx = cache_region.index(pos);
            match cache[idx] {
                Status::Uninitialized => {
                    let cpos = pos.reduce().div_floor(scalar(CHUNK_SIZE));
                    let tc = p.terrain_chunk(cpos);
                    let block_id = tc.blocks()[tc.bounds().index(pos)];
                    let block_name = bd.name(block_id);
                    trace!("  at {:?}, saw {} (inside? {})", pos, block_name,
                           block_name.starts_with(&*prefix));

                    if block_name.starts_with(&*prefix) {
                        cache[idx] = Status::Inside;
                        true
                    } else {
                        cache[idx] = Status::Outside;
                        false
                    }
                },
                Status::Inside => true,
                Status::Outside => false,
            }
        };

        const DIRS: [V3; 8] = [
            V3 { x:  0, y: -1, z: 0 },
            V3 { x: -1, y:  0, z: 0 },
            V3 { x:  0, y:  1, z: 0 },
            V3 { x:  1, y:  0, z: 0 },
            V3 { x: -1, y: -1, z: 0 },
            V3 { x: -1, y:  1, z: 0 },
            V3 { x:  1, y:  1, z: 0 },
            V3 { x:  1, y: -1, z: 0 },
        ];
        for pos in update_region.points() {
            trace!("checking {:?}", pos);
            if !is_inside(pos) {
                continue;
            }

            let mut bits = 0;
            for (i, &dir) in DIRS.iter().enumerate() {
                if is_inside(pos + dir) {
                    bits |= 1 << i;
                }
            }

            let part_name = INTERIOR_NAMES[INTERIOR_SHAPE_TABLE[bits] as usize];
            let name = format!("{}/{}", base, part_name);
            let block_id = unwrap!(bd.find_id(&*name));
            updates[update_region.index(pos)] = Some(block_id);
        }
    }

    {
        let mut p = wf.plane_mut(pid);

        for pos in update_region.points() {
            if let Some(block_id) = updates[update_region.index(pos)] {
                let cpos = pos.reduce().div_floor(scalar(CHUNK_SIZE));
                let mut tc = p.terrain_chunk_mut(cpos);
                tc.blocks_mut()[tc.bounds().index(pos)] = block_id;
            }
        }
    }

    for cpos in update_region.reduce().div_round_signed(CHUNK_SIZE).points() {
        let tcid = wf.world().plane(pid).terrain_chunk(cpos).id();
        wf.with_hooks(|h| h.on_terrain_chunk_update(tcid));
    }

    Ok(())
}

// Generated 2015-06-04 19:42:48 by util/gen_border_shape_table.py
const INTERIOR_SHAPE_TABLE: [u8; 256] = [
     0,  2,  4,  7,  3, 13, 11, 19,  1,  5, 14, 23,  9, 15, 27, 31,
     0,  2,  4,  8,  3, 13, 11, 21,  1,  5, 14, 25,  9, 15, 27, 39,
     0,  2,  4,  7,  3, 13, 12, 20,  1,  5, 14, 23,  9, 15, 29, 35,
     0,  2,  4,  8,  3, 13, 12, 22,  1,  5, 14, 25,  9, 15, 29, 43,
     0,  2,  4,  7,  3, 13, 11, 19,  1,  5, 14, 23, 10, 17, 28, 33,
     0,  2,  4,  8,  3, 13, 11, 21,  1,  5, 14, 25, 10, 17, 28, 41,
     0,  2,  4,  7,  3, 13, 12, 20,  1,  5, 14, 23, 10, 17, 30, 37,
     0,  2,  4,  8,  3, 13, 12, 22,  1,  5, 14, 25, 10, 17, 30, 45,
     0,  2,  4,  7,  3, 13, 11, 19,  1,  6, 14, 24,  9, 16, 27, 32,
     0,  2,  4,  8,  3, 13, 11, 21,  1,  6, 14, 26,  9, 16, 27, 40,
     0,  2,  4,  7,  3, 13, 12, 20,  1,  6, 14, 24,  9, 16, 29, 36,
     0,  2,  4,  8,  3, 13, 12, 22,  1,  6, 14, 26,  9, 16, 29, 44,
     0,  2,  4,  7,  3, 13, 11, 19,  1,  6, 14, 24, 10, 18, 28, 34,
     0,  2,  4,  8,  3, 13, 11, 21,  1,  6, 14, 26, 10, 18, 28, 42,
     0,  2,  4,  7,  3, 13, 12, 20,  1,  6, 14, 24, 10, 18, 30, 38,
     0,  2,  4,  8,  3, 13, 12, 22,  1,  6, 14, 26, 10, 18, 30, 46,
];

// Generated 2015-06-04 19:42:48 by util/gen_border_shape_table.py
const INTERIOR_NAMES: [&'static str; 47] = [
    "spot",
    "e",
    "n",
    "s",
    "w",
    "ne/0",
    "ne/1",
    "nw/0",
    "nw/1",
    "se/0",
    "se/1",
    "sw/0",
    "sw/1",
    "ns",
    "we",
    "nse/00",
    "nse/01",
    "nse/10",
    "nse/11",
    "nsw/00",
    "nsw/01",
    "nsw/10",
    "nsw/11",
    "nwe/00",
    "nwe/01",
    "nwe/10",
    "nwe/11",
    "swe/00",
    "swe/01",
    "swe/10",
    "swe/11",
    "nswe/0000",
    "nswe/0001",
    "nswe/0010",
    "nswe/0011",
    "nswe/0100",
    "nswe/0101",
    "nswe/0110",
    "nswe/0111",
    "nswe/1000",
    "nswe/1001",
    "nswe/1010",
    "nswe/1011",
    "nswe/1100",
    "nswe/1101",
    "nswe/1110",
    "nswe/1111",
];

