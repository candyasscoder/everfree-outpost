use rand::{Rng, XorShiftRng, SeedableRng};
use std::sync::mpsc::{Sender, Receiver};

use physics::CHUNK_SIZE;
use types::*;
use util::StrResult;

use data::Data;
use storage::Storage;
use terrain_gen::GenChunk;
use terrain_gen::dsc::DscGrid;
use terrain_gen::summary::Summary;


pub enum Command {
    Generate(Stable<PlaneId>, V2),
}

pub type Response = (Stable<PlaneId>, V2, GenChunk);

pub fn run(data: &Data,
           storage: &Storage,
           recv: Receiver<Command>,
           send: Sender<Response>) {
    let mut w = Worker::new(data, storage);

    for cmd in recv.iter() {
        use self::Command::*;
        match cmd {
            Generate(pid, cpos) => {
                let gc = w.generate_forest_chunk(pid, cpos);
                send.send((pid, cpos, gc)).unwrap();
            },
        }
    }
}


struct Worker<'d> {
    data: &'d Data,
    world_seed: u64,
    rng: XorShiftRng,
    summary: Summary<'d>,
}

impl<'d> Worker<'d> {
    fn new(data: &'d Data, storage: &'d Storage) -> Worker<'d> {
        Worker {
            data: data,
            world_seed: 0xe0e0e0e0_00012345,
            rng: SeedableRng::from_seed([0xe0e0e0e0,
                                         0x00012345,
                                         0xe0e0e0e0,
                                         0x00012345]),
            summary: Summary::new(storage),
        }
    }

    fn load_ds_levels(&mut self, pid: Stable<PlaneId>, cpos: V2) -> Option<&[u8]> {
        unwrap_or!(self.summary.load(pid, cpos).ok(), return None);
        Some(&self.summary.get(pid, cpos).ds_levels as &[u8])
    }

    fn generate_forest_ds_levels<R: Rng>(&mut self,
                                         rng: &mut R,
                                         pid: Stable<PlaneId>,
                                         cpos: V2) {
        let dist = cpos.abs().max();
        // 0 <= power < 16
        let power: u8 = power_from_dist(rng, dist);
        // `exp_power` grows exponentially with `power`.
        //  0 -  7 => 0
        //  8 - 11 => 1
        // 12 - 13 => 2
        // 14      => 3
        // 15      => 4
        let exp_power = (15 - power).leading_zeros() as u8 - (8 - 4);

        let mut grid = DscGrid::new(scalar(48), 4, |_pos, level, _phase| {
            if 3 - level <= exp_power { 1 } else { 0 }
        });


        let loaded_dirs = init_grid(scalar(CHUNK_SIZE), cpos,
                                    |cpos| self.load_ds_levels(pid, cpos),
                                    |pos, val| grid.set_value(pos, val));
        set_seed_ranges(&mut grid, scalar(CHUNK_SIZE),
                        |_pos| (100, 100 + power / 2));
        set_edge_constraints(&mut grid, scalar(CHUNK_SIZE));

        debug!("generate {:x} {:?}: loaded {:x}", pid.unwrap(), cpos, loaded_dirs);

        grid.fill(rng);

        // Save generated values to the summary.
        {
            let summ = self.summary.get_mut(pid, cpos);
            let bounds = Region::new(scalar(CHUNK_SIZE),
                                     scalar(2 * CHUNK_SIZE + 1));
            for pos in bounds.points() {
                let val = grid.get_value(pos).unwrap();
                summ.ds_levels[bounds.index(pos)] = val;
            }
        }
    }

    pub fn generate_forest_chunk(&mut self, pid: Stable<PlaneId>, cpos: V2) -> GenChunk {
        let seed: (u32, u32, u32, u32) = self.rng.gen();
        let mut rng: XorShiftRng = SeedableRng::from_seed([seed.0, seed.1, seed.2, seed.3]);
        debug!("generate {:x} {:?}: seed {:?}", pid.unwrap(), cpos, seed);

        self.summary.create(pid, cpos);
        self.generate_forest_ds_levels(&mut rng, pid, cpos);

        // Generate blocks.
        let summ = self.summary.get(pid, cpos);
        let bounds = Region::<V2>::new(scalar(0), scalar(CHUNK_SIZE));
        let get_level = |pos| summ.ds_levels[bounds.index(pos)];

        let mut gc = GenChunk::new();
        let block_data = &self.data.block_data;

        for pos in bounds.points() {
            let name = format!("grass/center/v{}", rng.gen_range(0, 4));
            gc.set_block(pos.extend(0), block_data.get_id(&name));

            let nw = get_level(pos + V2::new(0, 0)) as i32 - 100;
            let ne = get_level(pos + V2::new(1, 0)) as i32 - 100;
            let se = get_level(pos + V2::new(1, 1)) as i32 - 100;
            let sw = get_level(pos + V2::new(0, 1)) as i32 - 100;

            for z in (0 .. CHUNK_SIZE - 2).step_by(2) {
                // Rotate 180 degrees.  If the two south points are within the raised region, then
                // this is the *north* edge of the region.
                let bits = collect_bits(se >= z, sw >= z, nw >= z, ne >= z);

                if bits == 0 {
                    break;
                }

                let variant = BORDER_TILE_NAMES[bits as usize];
                let z0_id = block_data.get_id(&format!("cave/{}/z0", variant));
                let z1_id = block_data.get_id(&format!("cave/{}/z1", variant));
                let z2_id = block_data.get_id(&format!("cave_top/{}", variant));
                gc.set_block(pos.extend(z + 0), z0_id);
                gc.set_block(pos.extend(z + 1), z1_id);
                gc.set_block(pos.extend(z + 2), z2_id);
            }
        }

        gc
    }
}

fn div_rand_round<R: Rng>(r: &mut R, n: u32, d: u32) -> u32 {
    (n + r.gen_range(0, d)) / d
}

fn power_from_dist<R: Rng>(r: &mut R, dist: i32) -> u8 {
    fn ramp<R: Rng>(r: &mut R, x: u32, min_x: u32, max_x: u32, min_y: u32, max_y: u32) -> u32 {
        let x_range = max_x - min_x;
        let y_range = max_y - min_y;
        let dx = x - min_x;
        let dy = div_rand_round(r, dx * y_range, x_range);
        min_y + dy
    }

    let dist = dist as u32;
    if dist < 256 {
        ramp(r, dist, 0, 256, 0, 1) as u8
    } else if dist < 512 {
        ramp(r, dist, 256, 512, 1, 4) as u8
    } else if dist < 1024 {
        ramp(r, dist, 512, 1024, 4, 15) as u8
    } else {
        15
    }
}

/// Load values from adjacent pregenerated chunks.  `get_chunk(cpos)` should retrieve the contents
/// of the pregenerated chunk at `pos`, or `None` if that chunk has not been generated.  The
/// callback `set_value(pos, val)` will be called for each cell with pregenerated data.
fn init_grid<'a, GetChunk, SetValue>(size: V2,
                                     center: V2,
                                     mut get_chunk: GetChunk,
                                     mut set_value: SetValue) -> u8
        where GetChunk: FnMut(V2) -> Option<&'a [u8]>,
              SetValue: FnMut(V2, u8) {
    let mut loaded_dirs = 0;
    for (i, &dir) in DIRS.iter().enumerate() {
        let chunk = unwrap_or!(get_chunk(center + dir), continue);
        loaded_dirs |= 1 << i;

        let base = (dir + scalar(1)) * size;
        let bounds = Region::new(base, base + size + scalar(1));
        for pos in bounds.points() {
            set_value(pos, chunk[bounds.index(pos)]);
        }
    }
    loaded_dirs
}

/// Set ranges for all seed points. `f(pos)` should return the (low, high) range for the cell.
fn set_seed_ranges<F, GetRange>(grid: &mut DscGrid<F>,
                                size: V2,
                                mut get_range: GetRange)
        where GetRange: FnMut(V2) -> (u8, u8) {
    for step in Region::<V2>::new(scalar(0), scalar(4)).points() {
        let pos = step * size;
        match grid.get_value(pos) {
            Some(val) => grid.set_range(pos, val, val),
            None => {
                let (low, high) = get_range(pos);
                grid.set_range(pos, low, high)
            },
        }
    }
}

/// Apply constraints to edge points shared with pregenerated chunks.
fn set_edge_constraints<F>(grid: &mut DscGrid<F>,
                           size: V2) {
    let mut go = |pos| {
        if let Some(val) = grid.get_value(pos) {
            grid.set_constraint(pos, val, val);
        }
    };

    for x in 0 .. size.x + 1 {
        go(size + V2::new(x, 0));
        go(size + V2::new(x, size.y));
    }

    for y in 0 .. size.y + 1 {
        go(size + V2::new(0, y));
        go(size + V2::new(size.x, y));
    }
}






fn collect_bits(x0: bool, x1: bool, x2: bool, x3: bool) -> u8 {
    ((x0 as u8) << 0) |
    ((x1 as u8) << 1) |
    ((x2 as u8) << 2) |
    ((x3 as u8) << 3)
}

static DIRS: [V2; 8] = [
    V2 { x:  1, y:  0 },
    V2 { x:  1, y:  1 },
    V2 { x:  0, y:  1 },
    V2 { x: -1, y:  1 },
    V2 { x: -1, y:  0 },
    V2 { x: -1, y: -1 },
    V2 { x:  0, y: -1 },
    V2 { x:  1, y: -1 },
];

// Generated 2015-07-29 07:41:18 by util/gen_border_shape_table.py
const BORDER_TILE_NAMES: [&'static str; 16] = [
    "outside",
    "corner/outer/nw",
    "corner/outer/ne",
    "edge/n",
    "corner/outer/se",
    "cross/nw",
    "edge/e",
    "corner/inner/ne",
    "corner/outer/sw",
    "edge/w",
    "cross/ne",
    "corner/inner/nw",
    "edge/s",
    "corner/inner/sw",
    "corner/inner/se",
    "center",
];
