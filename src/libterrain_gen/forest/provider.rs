use rand::Rng;

use libserver_types::*;
use libphysics::CHUNK_SIZE;
use libserver_config::Data;
use libserver_config::Storage;

use {GenChunk, GenStructure};
use StdRng;
use cache::Cache;
use prop::LocalProperty;

use super::summary::ChunkSummary;
use super::summary::{SuperchunkSummary, SUPERCHUNK_SIZE};

use super::super_heightmap::SuperHeightmap;
use super::heightmap::Heightmap;
use super::caves::Caves;
use super::trees::Trees;
use super::treasure::Treasure;
use super::cliff_vaults::CliffVaults;


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

    fn generate_summary(&mut self,
                        pid: Stable<PlaneId>,
                        cpos: V2) {
        let height_grid = Heightmap::new(cpos, self.rng.gen(),
                                         |cpos| self.super_height(pid, cpos))
                              .generate_into(&mut self.cache, pid, cpos);

        Trees::new(self.rng.gen(), &height_grid)
            .generate_into(&mut self.cache, pid, cpos);

        CliffVaults::new(self.rng.gen(), &height_grid)
            .generate_into(&mut self.cache, pid, cpos);

        for layer in 0 .. CHUNK_SIZE as u8 / 2 {
            let layer_cutoff = layer * 2 + 100;

            let cave_grid = Caves::new(self.rng.gen(),
                                       layer,
                                       layer_cutoff,
                                       &height_grid)
                                .generate_into(&mut self.cache, pid, cpos);

            Treasure::new(self.rng.gen(),
                          layer,
                          &cave_grid)
                .generate_into(&mut self.cache, pid, cpos);
        }
    }


    pub fn generate(&mut self,
                    pid: Stable<PlaneId>,
                    cpos: V2) -> GenChunk {
        self.generate_summary(pid, cpos);


        let mut gc = GenChunk::new();
        let summ = self.cache.get(pid, cpos);
        // Bounds of the heightmap and cave grids, which assign a value to every vertex.
        let grid_bounds = Region::<V2>::new(scalar(0), scalar(CHUNK_SIZE + 1));
        // Bounds of the actual chunk, which assigns a block to every cell.
        let bounds = Region::<V2>::new(scalar(0), scalar(CHUNK_SIZE));

        let block_data = &self.data.block_data;
        macro_rules! block_id {
            ($($t:tt)*) => (block_data.get_id(&format!($($t)*)))
        };

        let structure_templates = &self.data.structure_templates;
        macro_rules! template_id {
            ($($t:tt)*) => (structure_templates.get_id(&format!($($t)*)))
        };

        let loot_tables = &self.data.loot_tables;
        let item_data = &self.data.item_data;

        // Grass layer
        let grass_ids = [
            block_id!("grass/center/v0"),
            block_id!("grass/center/v1"),
            block_id!("grass/center/v2"),
            block_id!("grass/center/v3"),
        ];
        for pos in bounds.points() {
            gc.set_block(pos.extend(0), *self.rng.choose(&grass_ids).unwrap());
        }

        // Cave/hill layers
        for layer in 0 .. CHUNK_SIZE as u8 / 2 {
            let floor_type = if layer == 0 { "grass" } else { "dirt" };

            for pos in bounds.points() {
                let (cave_key, top_key) = get_cell_keys(summ, pos, layer);
                if cave_key == OUTSIDE_KEY {
                    continue;
                }

                let layer_z = layer as i32 * 2;
                gc.set_block(pos.extend(layer_z + 0),
                             block_id!("cave/{}/z0/{}", cave_key, floor_type));
                gc.set_block(pos.extend(layer_z + 1),
                             block_id!("cave/{}/z1", cave_key));
                if layer_z + 2 < CHUNK_SIZE {
                    gc.set_block(pos.extend(layer_z + 2),
                                 block_id!("cave_top/{}", top_key));
                }
            }
        }

        // Cave entrances
        for &pos in &summ.cave_entrances {
            info!("placing entrance at {:?}", pos);
            let base = pos.reduce() - V2::new(3, 1);
            let layer = pos.z as u8 / 2;
            let floor_type = if layer == 0 { "grass" } else { "dirt" };

            for (i, &side) in ["left", "center", "right"].iter().enumerate() {
                let side_pos = base + V2::new(i as i32, 0);
                let (cave_key, _) = get_cell_keys(summ, side_pos, layer);
                gc.set_block(side_pos.extend(pos.z + 0),
                             block_id!("cave/entrance/{}/{}/z0/{}", side, cave_key, floor_type));
                gc.set_block(side_pos.extend(pos.z + 1),
                             block_id!("cave/entrance/{}/{}/z1", side, cave_key));
            }
        }

        // Natural ramps
        for &pos in &summ.natural_ramps {
            info!("placing ramp at {:?}", pos);
            let base = pos.reduce() - V2::new(3, 3);
            let layer = pos.z as u8 / 2;
            let floor_type = if layer == 0 { "grass" } else { "dirt" };
            for offset in Region::new(scalar(0), scalar(3)).points() {
                let (cave_key, _) = get_cell_keys(summ, base + offset, layer);
                info!("  {:?} => {}", offset, cave_key);
            }

            // Ramp
            gc.set_block((base + V2::new(1, 1)).extend(pos.z + 1),
                         block_id!("natural_ramp/ramp/z1"));
            gc.set_block((base + V2::new(1, 2)).extend(pos.z + 0),
                         block_id!("natural_ramp/ramp/z0/{}", floor_type));

            // Back of ramp
            let back_pos = base + V2::new(1, 0);
            let (cave_key, _) = get_cell_keys(summ, back_pos, layer);
            gc.set_block(back_pos.extend(pos.z + 1),
                         block_id!("natural_ramp/back/{}", cave_key));
            if pos.z + 2 < CHUNK_SIZE {
                gc.set_block(back_pos.extend(pos.z + 2),
                             block_id!("natural_ramp/top"));
            }

            const SIDE_BASE_KEY: u8 = 1*3*3 + 1*3*3*3;

            // Left side
            let left_pos = base + V2::new(0, 1);
            let (cave_key, _) = get_cell_keys(summ, left_pos, layer);
            gc.set_block(left_pos.extend(pos.z + 1),
                         block_id!("natural_ramp/left/{}/z1", cave_key));
            gc.set_block(left_pos.extend(pos.z + 0),
                         block_id!("cave/{}/z0/{}", SIDE_BASE_KEY, floor_type));

            // Right side
            let right_pos = base + V2::new(2, 1);
            let (cave_key, _) = get_cell_keys(summ, right_pos, layer);
            gc.set_block(right_pos.extend(pos.z + 1),
                         block_id!("natural_ramp/right/{}/z1", cave_key));
            gc.set_block(right_pos.extend(pos.z + 0),
                         block_id!("cave/{}/z0/{}", SIDE_BASE_KEY, floor_type));
        }


        // Trees/rocks
        for &pos in &self.cache.get(pid, cpos).tree_offsets {
            // Make sure the area near spawn is clear of structures.
            let abs_pos = pos + cpos * scalar(CHUNK_SIZE);
            if abs_pos.dot(abs_pos) < 5 * 5 {
                continue;
            }

            let height = summ.heightmap[grid_bounds.index(pos)];
            let layer = if height < 100 { 0 } else { (height - 100) / 2 + 1 };
            let z = layer as i32 * 2;

            let opt_id = if layer == 0 {
                loot_tables.eval_structure_table(&mut self.rng, "forest/floor")
            } else {
                loot_tables.eval_structure_table(&mut self.rng, "forest/hill")
            };

            if let Some(id) = opt_id {
                // TODO: filter trees/rocks during generation
                let gs = GenStructure::new(pos.extend(z), id);
                gc.structures.push(gs);
            }
        }

        // Treasure
        let chest_id = template_id!("chest");
        for layer in 0 .. CHUNK_SIZE as u8 / 2 {
            let layer_z = layer as i32 * 2;
            for &pos in &self.cache.get(pid, cpos).treasure_offsets[layer as usize] {
                let opt_id = loot_tables.eval_structure_table(&mut self.rng, "cave/floor");
                if let Some(id) = opt_id {
                    let mut gs = GenStructure::new(pos.extend(layer_z), id);
                    if id == chest_id {
                        let contents = loot_tables.eval_item_table(&mut self.rng, "cave/chest");
                        let mut s = String::new();
                        for (item_id, count) in contents {
                            s.push_str(&format!("{}:{},", item_data.name(item_id), count));
                        }
                        info!("generated chest, loot = {}", s);
                        gs.extra.insert("loot".to_owned(), s);
                    }
                    gc.structures.push(gs);
                }
            }
        }

        // Anvil (at spawn)
        if cpos == scalar(0) {
            let gs = GenStructure::new(scalar(0), template_id!("anvil"));
            gc.structures.push(gs);
        }

        gc
    }
}

pub fn cutoff(layer: u8) -> u8 {
    layer * 2 + 100
}

fn get_vertex_key(summ: &ChunkSummary, pos: V2, layer: u8) -> u8 {
    let bounds = Region::new(scalar(0), scalar(CHUNK_SIZE + 1));
    if summ.heightmap[bounds.index(pos)] < cutoff(layer) {
        // Outside the raised area
        1
    } else if !summ.cave_wall_layer(layer).get(bounds.index(pos)) {
        // Inside the raised area, and not a cave wall
        2
    } else {
        0
    }
}

fn get_cell_keys(summ: &ChunkSummary, pos: V2, layer: u8) -> (u8, u8) {
    let mut acc_cave = 0;
    let mut acc_top = 0;
    for &(dx, dy) in &[(0, 1), (1, 1), (1, 0), (0, 0)] {
        let val = get_vertex_key(summ, pos + V2::new(dx, dy), layer);
        acc_cave = acc_cave * 3 + val;
        acc_top = acc_top * 2 + (val != 1) as u8;
    }
    (acc_cave, acc_top)
}

const OUTSIDE_KEY: u8 = 1 + 1*3 + 1*3*3 + 1*3*3*3;
const CAVE_KEY: u8 = 2 + 2*3 + 2*3*3 + 2*3*3*3;
