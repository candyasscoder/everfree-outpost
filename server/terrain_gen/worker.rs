use rand::{Rng, XorShiftRng, SeedableRng, Rand};
use std::sync::mpsc::{Sender, Receiver};

use physics::{CHUNK_BITS, CHUNK_SIZE};
use types::*;
use util::StrResult;

use data::Data;
use storage::Storage;
use terrain_gen::GenChunk;
use terrain_gen::cellular::CellularGrid;
use terrain_gen::dsc::{DscGrid, Phase};
use terrain_gen::summary::Cache;
use terrain_gen::summary::{ChunkSummary, SuperchunkSummary};
use terrain_gen::summary::{SUPERCHUNK_BITS, SUPERCHUNK_SIZE};


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
    summary: Cache<'d, ChunkSummary>,
    super_summary: Cache<'d, SuperchunkSummary>,
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
            summary: Cache::new(storage, "chunk"),
            super_summary: Cache::new(storage, "superchunk"),
        }
    }

    fn generate_forest_super_ds_levels<R: Rng + Rand>(&mut self,
                                                      rng: &mut R,
                                                      pid: Stable<PlaneId>,
                                                      scpos: V2) {
        let mut rng2: R = rng.gen();
        let mut rng3: R = rng.gen();
        fn power<R: Rng>(rng: &mut R, cpos: V2) -> u8 {
            power_from_dist(rng, cpos.abs().max())
        }
        fn exp_power<R: Rng>(rng: &mut R, cpos: V2) -> u8 {
            (15 - power(rng, cpos)).leading_zeros() as u8 - (8 - 4)
        }

        let cpos = scpos * scalar(SUPERCHUNK_SIZE);
        let base = scalar(SUPERCHUNK_SIZE);
        let mut grid = DscGrid::new(scalar(SUPERCHUNK_SIZE * 3), SUPERCHUNK_BITS as u8, 
                                    |offset, level, _phase| {
                                        let ep = exp_power(&mut rng2, cpos + offset - base);
                                        if 4 - level <= ep { 2 } else { 1 }
                                    });
        let loaded_dirs = init_grid(scalar(SUPERCHUNK_SIZE), scpos,
                                    |scpos| self.load_super_ds_levels(pid, scpos),
                                    |pos, val| grid.set_range(pos, val, val));
        set_seed_ranges(&mut grid, scalar(SUPERCHUNK_SIZE),
                        |offset| {
                            let p = power(&mut rng3, cpos + offset - base);
                            info!("pos {:?}, power {}", cpos + offset - base, p);
                            (98, 100 + p / 2)
                        });
        set_edge_constraints(&mut grid, scalar(SUPERCHUNK_SIZE));

        debug!("generate(super) {:x} {:?}: loaded {:x}", pid.unwrap(), scpos, loaded_dirs);

        grid.fill(rng);

        // Save generated values to the summary.
        {
            let summ = self.super_summary.get_mut(pid, scpos);
            let bounds = Region::new(scalar(SUPERCHUNK_SIZE),
                                     scalar(2 * SUPERCHUNK_SIZE + 1));
            for pos in bounds.points() {
                let val = grid.get_value(pos).unwrap();
                summ.ds_levels[bounds.index(pos)] = val;
            }
        }
    }

    fn load_super_ds_levels(&mut self, pid: Stable<PlaneId>, scpos: V2) -> Option<&[u8]> {
        unwrap_or!(self.super_summary.load(pid, scpos).ok(), return None);
        Some(&self.super_summary.get(pid, scpos).ds_levels as &[u8])
    }

    fn super_ds_levels(&mut self, pid: Stable<PlaneId>, scpos: V2) -> &[u8] {
        info!("look up {} {:?}", pid.unwrap(), scpos);
        if self.super_summary.load(pid, scpos).is_err() {
            let seed: (u32, u32, u32, u32) = self.rng.gen();
            let mut rng: XorShiftRng = SeedableRng::from_seed([seed.0, seed.1, seed.2, seed.3]);
            debug!("generate(super) {:x} {:?}: seed {:?}", pid.unwrap(), scpos, seed);

            self.super_summary.create(pid, scpos);
            self.generate_forest_super_ds_levels(&mut rng, pid, scpos);
        }
        &self.super_summary.get(pid, scpos).ds_levels as &[u8]
    }

    fn chunk_super_ds_level(&mut self, pid: Stable<PlaneId>, cpos: V2) -> u8 {
        if cpos == scalar(0) {
            return 98;
        }

        let scpos = cpos.div_floor(scalar(SUPERCHUNK_SIZE));
        let base = scpos * scalar(SUPERCHUNK_SIZE);
        let bounds = Region::new(base, base + scalar(SUPERCHUNK_SIZE + 1));
        let ds_levels = self.super_ds_levels(pid, scpos);
        info!("referenced super {:?} for {:?}", scpos, cpos);
        ds_levels[bounds.index(cpos)]
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

        let mut grid = DscGrid::new(scalar(CHUNK_SIZE * 3), CHUNK_BITS as u8,
                                    |_pos, level, phase| { 
                                        if level == 3 && phase == Phase::Square { 1 } else { 0 }
                                    });


        let loaded_dirs = init_grid(scalar(CHUNK_SIZE), cpos,
                                    |cpos| self.load_ds_levels(pid, cpos),
                                    |pos, val| grid.set_range(pos, val, val));
        set_seed_ranges(&mut grid, scalar(CHUNK_SIZE),
                        |offset| {
                            let cpos = cpos - scalar(1) + offset.div_floor(scalar(CHUNK_SIZE));
                            let level = self.chunk_super_ds_level(pid, cpos);
                            (level - 1, level)
                        });
        set_edge_constraints(&mut grid, scalar(CHUNK_SIZE));

        debug!("generate {:x} {:?}: loaded {:x}", pid.unwrap(), cpos, loaded_dirs);

        grid.fill(rng);
        grid.debug();

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

    fn load_cave_edges(&mut self,
                       pid: Stable<PlaneId>,
                       cpos: V2,
                       layer: u8,
                       edge: u8) -> Option<&[u8]> {
        unwrap_or!(self.summary.load(pid, cpos).ok(), return None);
        Some(&self.summary.get(pid, cpos).cave_nums[layer as usize][edge as usize] as &[u8])
    }

    fn generate_cave_layer<R: Rng>(&mut self,
                                   rng: &mut R,
                                   pid: Stable<PlaneId>,
                                   cpos: V2,
                                   layer: u8) -> CellularGrid {
        let mut grid = CellularGrid::new(scalar(CHUNK_SIZE + 1));

        init_grid_edges(scalar(CHUNK_SIZE), cpos,
                        |cpos, edge| self.load_cave_edges(pid, cpos, layer, edge),
                        |pos, val| grid.set_fixed(pos, val));
        {
            let ds_levels = self.load_ds_levels(pid, cpos).unwrap();
            let bounds = Region::new(scalar(0), scalar(CHUNK_SIZE + 1));
            init_grid_center(scalar(CHUNK_SIZE),
                             layer * 2,
                             |pos| ds_levels[bounds.index(pos)] as i32 - 100,
                             |pos| grid.set_fixed(pos, true));
        }
        grid.init(|_pos| rng.gen_range(0, 3) < 1);

        /*
        info!("original layer dump =====");
        for i in 0 .. CHUNK_SIZE + 1 {
            let mut s = String::new();
            for j in 0 .. CHUNK_SIZE + 1 {
                s.push_str(if grid.get(V2::new(j, i)) { "#" } else { "." });
            }
            info!("> {}", s);
        }
        info!("===== end dump");
        */

        for _ in 0 .. 3 {
            grid.step(|here, active, total| 2 * (here as u8 + active) > total);
        }

        {
            let summ = self.summary.get_mut(pid, cpos);
            let layer_idx = layer as usize;
            for i in 0 .. CHUNK_SIZE + 1 {
                let idx = i as usize;
                summ.cave_nums[layer_idx][0][idx] = grid.get(V2::new(i, 0)) as u8;
                summ.cave_nums[layer_idx][1][idx] = grid.get(V2::new(CHUNK_SIZE, i)) as u8;
                summ.cave_nums[layer_idx][2][idx] = grid.get(V2::new(i, CHUNK_SIZE)) as u8;
                summ.cave_nums[layer_idx][3][idx] = grid.get(V2::new(0, i)) as u8;
            }
        }

        /*
        info!("layer dump =====");
        for i in 0 .. CHUNK_SIZE + 1 {
            let mut s = String::new();
            for j in 0 .. CHUNK_SIZE + 1 {
                s.push_str(if grid.get(V2::new(j, i)) { "#" } else { "." });
            }
            info!("> {}", s);
        }
        info!("===== end dump");
        */

        grid
    }

    pub fn generate_forest_chunk(&mut self, pid: Stable<PlaneId>, cpos: V2) -> GenChunk {
        let seed: (u32, u32, u32, u32) = self.rng.gen();
        let mut rng: XorShiftRng = SeedableRng::from_seed([seed.0, seed.1, seed.2, seed.3]);
        debug!("generate {:x} {:?}: seed {:?}", pid.unwrap(), cpos, seed);

        self.summary.create(pid, cpos);
        self.generate_forest_ds_levels(&mut rng, pid, cpos);

        // Generate blocks.
        let bounds = Region::<V2>::new(scalar(0), scalar(CHUNK_SIZE));
        let bounds_inc = Region::<V2>::new(scalar(0), scalar(CHUNK_SIZE + 1));

        let mut gc = GenChunk::new();
        let block_data = &self.data.block_data;

        let grass_ids = (0 .. 4).map(|i| format!("grass/center/v{}", i))
                                .map(|s| block_data.get_id(&s))
                                .collect::<Vec<_>>();
        const OUTSIDE_KEY: u8 = 1 + 3 * (1 + 3 * (1 + 3 * (1)));

        for layer in 0 .. CHUNK_SIZE / 2 {
            let layer_z = 2 * layer;
            let layer = layer as u8;

            let cave_grid = self.generate_cave_layer(&mut rng, pid, cpos, layer);

            let summ = self.summary.get(pid, cpos);
            let mut level_grid = CellularGrid::new(scalar(CHUNK_SIZE + 1));
            level_grid.init(|pos| summ.ds_levels[bounds_inc.index(pos)] as i32 - 100 >= layer_z);

            let floor_type = if layer == 0 { "grass" } else { "dirt" };

            for pos in bounds.points() {
                if layer == 0 {
                    gc.set_block(pos.extend(layer_z), *rng.choose(&grass_ids).unwrap())
                }
                let key = collect_indexes(pos, &level_grid, &cave_grid);

                if key == OUTSIDE_KEY {
                    continue;
                }

                let z0_id = block_data.get_id(&format!("cave/{}/z0/{}", key, floor_type));
                let z1_id = block_data.get_id(&format!("cave/{}/z1", key));
                // TODO
                //let z2_id = block_data.get_id(&format!("cave_top/{}", variant));
                gc.set_block(pos.extend(layer_z + 0), z0_id);
                gc.set_block(pos.extend(layer_z + 1), z1_id);
                //gc.set_block(pos.extend(z + 2), z2_id);
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
        if grid.get_range(pos).is_none() {
            let (low, high) = get_range(pos);
            grid.set_range(pos, low, high)
        }
    }
}

/// Apply constraints to edge points shared with pregenerated chunks.
fn set_edge_constraints<F>(grid: &mut DscGrid<F>,
                           size: V2) {
    let mut go = |pos| {
        if let Some((min, max)) = grid.get_range(pos) {
            grid.set_constrained(pos);
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


fn init_grid_edge<SetValue>(size: i32,
                            edge: &[u8],
                            mut set_value: SetValue)
        where SetValue: FnMut(i32, bool) {
    for i in 0 .. size + 1 {
        set_value(i, edge[i as usize] != 0);
    }
}

fn init_grid_edges<'a, GetEdge, SetValue>(size: V2,
                                          center: V2,
                                          mut get_edge: GetEdge,
                                          mut set_value: SetValue)
        where GetEdge: FnMut(V2, u8) -> Option<&'a [u8]>,
              SetValue: FnMut(V2, bool) {
    get_edge(center + V2::new( 0, -1), 2)
        .map(|e| init_grid_edge(size.x, e, |i, v| set_value(V2::new(i, 0), v)));
    get_edge(center + V2::new( 0,  1), 0)
        .map(|e| init_grid_edge(size.x, e, |i, v| set_value(V2::new(i, size.x), v)));
    get_edge(center + V2::new(-1,  0), 1)
        .map(|e| init_grid_edge(size.y, e, |i, v| set_value(V2::new(0, i), v)));
    get_edge(center + V2::new( 1,  0), 3)
        .map(|e| init_grid_edge(size.y, e, |i, v| set_value(V2::new(size.x, i), v)));

    get_edge(center + V2::new(-1, -1), 2)
        .map(|e| set_value(V2::new(0, 0), e[size.x as usize] != 0));
    get_edge(center + V2::new(-1,  1), 0)
        .map(|e| set_value(V2::new(0, size.y), e[size.x as usize] != 0));
    get_edge(center + V2::new( 1, -1), 2)
        .map(|e| set_value(V2::new(size.x, 0), e[0] != 0));
    get_edge(center + V2::new( 1,  1), 0)
        .map(|e| set_value(V2::new(size.x, size.y), e[0] != 0));
}

fn init_grid_center<GetLevel, SetValue>(size: V2,
                                        z: u8,
                                        mut get_level: GetLevel,
                                        mut set_value: SetValue)
        where GetLevel: FnMut(V2) -> i32,
              SetValue: FnMut(V2) {
    let z = z as i32;
    let bounds = Region::new(scalar(0), size + scalar(1));
    for pos in bounds.points() {
        if get_level(pos) < z {
            set_value(pos);
        }
    }
}






fn collect_bits(x0: bool, x1: bool, x2: bool, x3: bool) -> u8 {
    ((x0 as u8) << 0) |
    ((x1 as u8) << 1) |
    ((x2 as u8) << 2) |
    ((x3 as u8) << 3)
}

fn collect_indexes(base: V2,
                   level_grid: &CellularGrid,
                   cave_grid: &CellularGrid) -> u8 {
    let mut acc = 0;
    for &(x, y) in &[(0, 1), (1, 1), (1, 0), (0, 0)] {
        let pos = base + V2::new(x, y);
        let val =
            if !level_grid.get(pos) {
                // 0 in level_grid = outside the raised area for this level
                1
            } else if !cave_grid.get(pos) {
                // 0 in cave_grid = open space inside cave (1 = wall)
                2
            } else {
                0
            };
        acc = acc * 3 + val;
    }
    acc
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
