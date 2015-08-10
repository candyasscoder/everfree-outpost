use rand::Rng;

use types::*;
use physics::CHUNK_SIZE;
use physics::Shape;

use data::Data;
use storage::Storage;
use terrain_gen::{GenChunk, GenStructure};
use terrain_gen::StdRng;
use terrain_gen::cache::Cache;
use terrain_gen::cellular::CellularGrid;
use terrain_gen::dsc::DscGrid;
use terrain_gen::prop::{self, LocalProperty};

use super::summary::ChunkSummary;
use super::summary::{SuperchunkSummary, SUPERCHUNK_SIZE};

use super::super_heightmap::SuperHeightmap;
use super::heightmap::Heightmap;
use super::caves::Caves;
use super::trees::Trees;


pub struct Provider<'d> {
    data: &'d Data,
    rng: StdRng,
    cache: Cache<'d, ChunkSummary>,
    super_cache: Cache<'d, SuperchunkSummary>,
}

impl<'d> Provider<'d> {
    pub fn new(data: &'d Data, storage: &'d Storage, rng: StdRng) -> Provider<'d> {
        Provider {
            data: data,
            rng: rng,
            cache: Cache::new(storage, "chunk"),
            super_cache: Cache::new(storage, "superchunk"),
        }
    }

    fn get_super_heightmap(&mut self,
                           pid: Stable<PlaneId>,
                           scpos: V2) -> &[u8] {
        if let Err(_) = self.super_cache.load(pid, scpos) {
            SuperHeightmap::new(scpos, self.rng.gen())
                .generate_into(&mut self.super_cache, pid, scpos);
        }
        &self.super_cache.get(pid, scpos).ds_levels
    }

    fn super_height(&mut self,
                    pid: Stable<PlaneId>,
                    cpos: V2) -> u8 {
        if cpos == scalar(0){
            return 98;
        }

        let scpos = cpos.div_floor(scalar(SUPERCHUNK_SIZE));
        let base = scpos * scalar(SUPERCHUNK_SIZE);
        let bounds = Region::new(base, base + scalar(SUPERCHUNK_SIZE + 1));
        let heightmap = self.get_super_heightmap(pid, scpos);
        heightmap[bounds.index(cpos)]
    }

    pub fn generate(&mut self,
                    pid: Stable<PlaneId>,
                    cpos: V2) -> GenChunk {
        let mut gc = GenChunk::new();

        let height_grid = Heightmap::new(cpos, self.rng.gen(),
                                         |cpos| self.super_height(pid, cpos))
                              .generate_into(&mut self.cache, pid, cpos);

        Trees::new(self.rng.gen())
            .generate_into(&mut self.cache, pid, cpos);

        for layer in 0 .. CHUNK_SIZE as u8 / 2 {
            let layer_cutoff = layer * 2 + 100;

            let opt_entrance = self.place_entrance(&height_grid,
                                                   layer_cutoff);
            let cave_grid = Caves::new(self.rng.gen(),
                                       layer,
                                       layer_cutoff,
                                       &height_grid,
                                       opt_entrance.as_slice())
                                .generate_into(&mut self.cache, pid, cpos);

            self.fill_layer(&mut gc,
                            layer,
                            layer_cutoff,
                            &height_grid,
                            opt_entrance,
                            &cave_grid);
        }

        let base = scalar(CHUNK_SIZE);
        let tree_id = self.data.structure_templates.get_id("tree");
        let rock_id = self.data.structure_templates.get_id("rock");
        for &pos in &self.cache.get(pid, cpos).tree_offsets {
            let id = if self.rng.gen_range(0, 3) < 2 { tree_id } else { rock_id };
            let template = self.data.structure_templates.template(id);
            let footprint = Region::new(pos, pos + template.size.reduce());
            let max_height = footprint.points_inclusive()
                                      .map(|p| height_grid.get_value(p + base).unwrap())
                                      .max().unwrap();
            let max_layer = if max_height < 100 { 0 } else { (max_height - 100) / 2 + 1 };
            let z = max_layer as i32 * 2;
            // TODO: check (or constrain) heights of neighboring chunks.  Right now we can
            // mistakenly place a tree off the edge of a cliff if the cliff is at x=0 on the
            // neighboring chunk.
            let ok = footprint.intersect(Region::new(scalar(0), scalar(CHUNK_SIZE)))
                              .points()
                              .all(|p| {
                                  let id = gc.get_block(p.extend(z));
                                  self.data.block_data.shape(id) == Shape::Floor
                              });

            if ok {
                let gs = GenStructure::new(pos.extend(z), id);
                gc.structures.push(gs);
            }
        }

        gc
    }

    fn place_entrance(&mut self, height_grid: &DscGrid, cutoff: u8) -> Option<V2> {
        // Entrance requirements:
        //  >= >  >  >=
        //  == == == ==
        const ENTRANCE_PATTERN: u32 = (0b_00_01_01_00 << 10) |
                                      (0b_00_00_00_00 <<  0);
        const ENTRANCE_MASK: u32 =    (0b_10_11_11_10 << 10) |
                                      (0b_11_11_11_11 <<  0);
        let candidates = find_pattern(height_grid, cutoff, ENTRANCE_PATTERN, ENTRANCE_MASK);
        // Results listed in `candidates` are "x".  Translate to get "*".
        // _ * _ _
        // _ _ _ x
        self.rng.choose(&candidates).map(|&pos| pos - V2::new(2, 1))
    }

    fn fill_layer(&mut self,
                  gc: &mut GenChunk,
                  layer: u8,
                  cutoff: u8,
                  height_grid: &DscGrid,
                  opt_entrance: Option<V2>,
                  cave_grid: &CellularGrid) {
        let base: V2 = scalar(CHUNK_SIZE);

        let get = |pos| {
            let val = height_grid.get_value(base + pos).unwrap();
            if val < cutoff {
                // Outside the raised portion.
                1
            } else if !cave_grid.get(base + pos) {
                // Inside raised portion, but not a wall.
                2
            } else {
                0
            }
        };

        let get_key = |pos| {
            let mut acc_cave = 0;
            let mut acc_top = 0;
            for &(dx, dy) in &[(0, 1), (1, 1), (1, 0), (0, 0)] {
                let val = get(pos + V2::new(dx, dy));
                acc_cave = acc_cave * 3 + val;
                acc_top = acc_top * 2 + (val != 1) as u8;
            }
            (acc_cave, acc_top)
        };
        const OUTSIDE_KEY: u8 = 1 + 1*3 + 1*3*3 + 1*3*3*3;

        let block_data = &self.data.block_data;
        macro_rules! block_id {
            ($($t:tt)*) => (block_data.get_id(&format!($($t)*)))
        };

        let layer_z = 2 * layer as i32;
        let floor_type = if layer == 0 { "grass" } else { "dirt" };
        for pos in Region::<V2>::new(scalar(0), scalar(CHUNK_SIZE)).points() {
            if layer == 0 {
                gc.set_block(pos.extend(layer_z),
                             block_id!("grass/center/v{}", self.rng.gen_range(0, 4)));
            }

            let (cave_key, top_key) = get_key(pos);
            if cave_key == OUTSIDE_KEY {
                continue;
            }

            gc.set_block(pos.extend(layer_z + 0),
                         block_id!("cave/{}/z0/{}", cave_key, floor_type));
            gc.set_block(pos.extend(layer_z + 1),
                         block_id!("cave/{}/z1", cave_key));
            if layer_z + 2 < CHUNK_SIZE {
                gc.set_block(pos.extend(layer_z + 2),
                             block_id!("cave_top/{}", top_key));
            }
        }

        if let Some(epos) = opt_entrance {
            info!("placing entrance at {:?}", epos);
            for (i, &side) in ["left", "center", "right"].iter().enumerate() {
                // Note that `epos` points to the middle cell, not the left one.
                let pos = epos + V2::new(i as i32 - 1, 0);
                let (cave_key, _) = get_key(pos);
                gc.set_block(pos.extend(layer_z + 0),
                             block_id!("cave/entrance/{}/{}/z0/{}", side, cave_key, floor_type));
                gc.set_block(pos.extend(layer_z + 1),
                             block_id!("cave/entrance/{}/{}/z1", side, cave_key));
            }
        }
    }
}

fn find_pattern(grid: &DscGrid, cutoff: u8, bits: u32, mask: u32) -> Vec<V2> {
    let base: V2 = scalar(CHUNK_SIZE);
    let get = |x, y| {
        if y < 0 {
            return 0;
        }
        let pos = base + V2::new(x, y);
        let val = grid.get_value(pos).unwrap();

        let above = val >= cutoff;
        let below = val + 2 < cutoff;
        (above as u32) | ((below as u32) << 1)
    };

    // Accumulator records a 4x3 region above and to the left of the current point.  It
    // consists of three sections, each containing four 2-bit fields plus 2 bits of padding.
    // The lower bit of each field is a 1 if the height of the corresponding cell is above the
    // current level, and the upper bit is 1 if it is strictly below the current level.  If
    // both are zero, then the cell is exactly on the current level.
    //
    //            30             20             10              0 
    //   high ->  __ __ AA BB CC DD __ EE FF GG HH __ II JJ KK LL  <- low
    //
    // Grid:
    //      ABCD
    //      EFGH
    //      IJKL <- current cell
    let mut acc = 0_u32;
    let mut result = Vec::new();

    for y in 0 .. CHUNK_SIZE + 1 {
        acc = 0;
        for x in 0 .. CHUNK_SIZE + 1 {
            acc <<= 2;
            acc &= !((3 << 8) | (3 << 18) | (3 << 28));    // Clear padding.
            acc |= get(x, y - 2) << 20;
            acc |= get(x, y - 1) << 10;
            acc |= get(x, y - 0) <<  0;

            if x >= 3 && y >= 1 && acc & mask == bits {
                result.push(V2::new(x, y));
            }
        }
    }
    result
}

/*
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

            let entrances = self.place_entrances(pid, cpos, layer);
            let entrance_pos = rng.choose(&entrances).map(|&x| x);
            let cave_grid = self.generate_cave_layer(&mut rng, pid, cpos, layer, entrance_pos);

            let summ = self.summary.get(pid, cpos);
            let mut level_grid = CellularGrid::new(scalar(CHUNK_SIZE + 1));
            level_grid.init(|pos| summ.ds_levels[bounds_inc.index(pos)] as i32 - 100 >= layer_z);

            let floor_type = if layer == 0 { "grass" } else { "dirt" };

            for pos in bounds.points() {
                if layer == 0 {
                    gc.set_block(pos.extend(layer_z), *rng.choose(&grass_ids).unwrap())
                }
                let (key, top_key) = collect_indexes(pos, &level_grid, &cave_grid);

                if key == OUTSIDE_KEY {
                    continue;
                }

                let z0_id = block_data.get_id(&format!("cave/{}/z0/{}", key, floor_type));
                let z1_id = block_data.get_id(&format!("cave/{}/z1", key));
                gc.set_block(pos.extend(layer_z + 0), z0_id);
                gc.set_block(pos.extend(layer_z + 1), z1_id);
                if layer_z + 2 < CHUNK_SIZE {
                    let z2_id = block_data.get_id(&format!("cave_top/{}", top_key));
                    gc.set_block(pos.extend(layer_z + 2), z2_id);
                }
            }

            if let Some(epos) = entrance_pos {
                for (i, &side) in ["left", "center", "right"].iter().enumerate() {
                    let pos = epos + V2::new(i as i32, 0);
                    let (key, _) = collect_indexes(pos, &level_grid, &cave_grid);
                    let z0_id = block_data.get_id(&format!("cave/entrance/{}/{}/z0/{}",
                                                           side, key, floor_type));
                    let z1_id = block_data.get_id(&format!("cave/entrance/{}/{}/z1",
                                                           side, key));
                    gc.set_block(pos.extend(layer_z + 0), z0_id);
                    gc.set_block(pos.extend(layer_z + 1), z1_id);
                }
            }
        }

        gc
*/
