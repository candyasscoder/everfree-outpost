use std::collections::HashMap;
use std::hash::{Hash, Hasher, SipHasher};
use rand::{Rng, XorShiftRng, SeedableRng};

use physics::CHUNK_SIZE;
use types::*;
use util::StrResult;

use data::Data;
use storage::Storage;
use world::Fragment as World_Fragment;
use world::object::*;

pub use self::disk_sampler::IsoDiskSampler;
pub use self::diamond_square::DiamondSquare;
pub use self::fields::{ConstantField, RandomField, FilterField, BorderField};

use self::summary::Summary;
use self::dsc::DscGrid;

mod diamond_square;
mod disk_sampler;
mod fields;
mod summary;
mod dsc;


pub struct TerrainGen<'d> {
    data: &'d Data,
    world_seed: u64,
    rng: XorShiftRng,
    summary: Summary<'d>,
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

impl<'d> TerrainGen<'d> {
    pub fn new(data: &'d Data, storage: &'d Storage) -> TerrainGen<'d> {
        TerrainGen {
            data: data,
            world_seed: 0xe0e0e0e0_00012345,
            rng: SeedableRng::from_seed([0xe0e0e0e0,
                                         0x00012345,
                                         0xe0e0e0e0,
                                         0x00012345]),
            summary: Summary::new(storage),
        }
    }

    pub fn data(&self) -> &'d Data {
        self.data
    }

    pub fn plane_rng(&self, pid: Stable<PlaneId>, seed: u32) -> XorShiftRng {
        let mut hasher = SipHasher::new_with_keys(self.world_seed, 0xac87_2554_6d5c_bc1f);
        (pid.unwrap(), seed).hash(&mut hasher);
        let seed0 = hasher.finish();

        SeedableRng::from_seed([(seed0 >> 32) as u32,
                                seed0 as u32,
                                0xa21b_0552,
                                0x204c_17f8])
    }

    pub fn chunk_rng(&self, pid: Stable<PlaneId>, cpos: V2, seed: u32) -> XorShiftRng {
        // TODO: temporary hack to avoid regenerating terrain in PLANE_FOREST
        if pid == STABLE_PLANE_FOREST {
            SeedableRng::from_seed([self.world_seed as u32 ^ 0xfaa3e2a2,
                                    cpos.x as u32,
                                    cpos.y as u32,
                                    seed])
        } else {
            let mut hasher = SipHasher::new_with_keys(self.world_seed, 0xb953_9155_1d94_626c);
            (pid.unwrap(), cpos, seed).hash(&mut hasher);
            let seed0 = hasher.finish();

            SeedableRng::from_seed([(seed0 >> 32) as u32,
                                    seed0 as u32,
                                    0x7307_3120,
                                    0x7f68_4998])
        }
    }

    pub fn rng(&self, seed: u32) -> XorShiftRng {
        // TODO: make this use all of world_seed
        SeedableRng::from_seed([self.world_seed as u32 ^ 0x3ba6d154,
                                0x34c9c7b1,
                                0xf8499a88,
                                seed])
    }

    pub fn generate_forest_chunk(&mut self, pid: Stable<PlaneId>, cpos: V2) -> GenChunk {
        let mut rng = self.rng.gen::<XorShiftRng>();

        let mut grid = DscGrid::new(scalar(48), 4, |_pos, level, _phase| {
            match level {
                3 => 2,
                2 => 1,
                _ => 0,
            }
        });

        // Load values from adjacent pregenerated chunks.
        for &dir in &DIRS {
            match self.summary.load(pid, cpos + dir) {
                Ok(_) => {},
                Err(_) => continue,
            };
            let summ = self.summary.get(pid, cpos + dir);

            let base = (dir + scalar(1)) * scalar(CHUNK_SIZE);
            let bounds = Region::new(base, base + scalar(CHUNK_SIZE + 1));
            for pos in bounds.points() {
                let val = summ.ds_levels[bounds.index(pos)];
                grid.set_value(pos, val);
            }
        }

        // Set ranges for all seed points.
        for step in Region::<V2>::new(scalar(0), scalar(4)).points() {
            let pos = step * scalar(CHUNK_SIZE);
            match grid.get_value(pos) {
                Some(val) => grid.set_range(pos, val, val),
                None => grid.set_range(pos, 100, 105),
            }
        }

        // Apply constraints to edge points shared with pregenerated chunks.
        for i in 0 .. CHUNK_SIZE + 1 {
            for &(x, y) in &[(i, 0), (0, i), (i, CHUNK_SIZE), (CHUNK_SIZE, i)] {
                let pos = V2::new(x, y) + scalar(CHUNK_SIZE);
                if let Some(val) = grid.get_value(pos) {
                    grid.set_constraint(pos, val, val);
                }
            }
        }

        // Go!
        grid.fill(&mut rng);

        // Save generated values to the summary.
        {
            let summ = self.summary.create(pid, cpos);

            let bounds = Region::new(scalar(CHUNK_SIZE),
                                     scalar(2 * CHUNK_SIZE + 1));
            for pos in bounds.points() {
                debug!("{:?}", pos);
                let val = grid.get_value(pos).unwrap();
                summ.ds_levels[bounds.index(pos)] = val;
            }
        }

        // Generate blocks.
        let mut gc = GenChunk::new();
        let block_data = &self.data.block_data;

        for pos in Region::<V2>::new(scalar(0), scalar(CHUNK_SIZE)).points() {
            let name = format!("grass/center/v{}", rng.gen_range(0, 4));
            gc.set_block(pos.extend(0), block_data.get_id(&name));
        }

        let base = scalar::<V2>(CHUNK_SIZE);
        for pos in Region::<V2>::new(scalar(0), scalar(CHUNK_SIZE)).points() {
            let nw = grid.get_value(base + pos + V2::new(0, 0)).unwrap() as i32 - 100;
            let ne = grid.get_value(base + pos + V2::new(1, 0)).unwrap() as i32 - 100;
            let se = grid.get_value(base + pos + V2::new(1, 1)).unwrap() as i32 - 100;
            let sw = grid.get_value(base + pos + V2::new(0, 1)).unwrap() as i32 - 100;

            for z in (0 .. CHUNK_SIZE).step_by(2) {
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

fn collect_bits(x0: bool, x1: bool, x2: bool, x3: bool) -> u8 {
    ((x0 as u8) << 0) |
    ((x1 as u8) << 1) |
    ((x2 as u8) << 2) |
    ((x3 as u8) << 3)
}

pub trait Fragment<'d> {
    fn terrain_gen_mut(&mut self) -> &mut TerrainGen<'d>;

    type WF: World_Fragment<'d>;
    fn with_world<F, R>(&mut self, f: F) -> R
            where F: FnOnce(&mut Self::WF) -> R;

    fn generate(&mut self,
                pid: PlaneId,
                cpos: V2) -> StrResult<TerrainChunkId> {
        let stable_pid = self.with_world(|wf| wf.plane_mut(pid).stable_id());
        let gc = self.terrain_gen_mut().generate_forest_chunk(stable_pid, cpos);
        self.with_world(move |wf| wf.create_terrain_chunk(pid, cpos, gc.blocks).map(|tc| tc.id()))
    }
}


pub struct GenChunk {
    pub blocks: Box<BlockChunk>,
    pub structures: Vec<GenStructure>,
}

impl GenChunk {
    pub fn new() -> GenChunk {
        GenChunk {
            blocks: Box::new(EMPTY_CHUNK),
            structures: Vec::new(),
        }
    }

    pub fn set_block(&mut self, pos: V3, val: BlockId) {
        let bounds = Region::new(scalar(0), scalar(CHUNK_SIZE));
        assert!(bounds.contains(pos));
        self.blocks[bounds.index(pos)] = val;
    }
}

pub struct GenStructure {
    pub pos: V3,
    pub template: TemplateId,
    pub extra: HashMap<String, String>,
}

impl GenStructure {
    pub fn new(pos: V3, template: TemplateId) -> GenStructure {
        GenStructure {
            pos: pos,
            template: template,
            extra: HashMap::new(),
        }
    }
}


pub trait PointSource {
    fn generate_points(&self, bounds: Region2) -> Vec<V2>;
}

pub trait Field {
    fn get_value(&self, pos: V2) -> i32;

    fn get_region(&self, bounds: Region2, buf: &mut [i32]) {
        for p in bounds.points() {
            let idx = bounds.index(p);
            buf[idx] = self.get_value(p);
        }
    }
}

impl Field for Box<Field> {
    fn get_value(&self, pos: V2) -> i32 {
        (**self).get_value(pos)
    }

    fn get_region(&self, bounds: Region2, buf: &mut [i32]) {
        (**self).get_region(bounds, buf)
    }
}


struct PointRng {
    seed: u64,
    pos: V2,
    extra: u32,
    counter: u32,
}

impl PointRng {
    pub fn new(seed: u64, pos: V2, extra: u32) -> PointRng {
        PointRng {
            seed: seed,
            pos: pos,
            extra: extra,
            counter: 0,
        }
    }
}

impl Rng for PointRng {
    fn next_u32(&mut self) -> u32 {
        self.next_u64() as u32
    }

    fn next_u64(&mut self) -> u64 {
        let mut hasher = SipHasher::new_with_keys(self.seed, 0x9aa64385cac2f793);
        (self.pos, self.extra, self.counter).hash(&mut hasher);
        self.counter += 1;
        hasher.finish()
    }
}


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
