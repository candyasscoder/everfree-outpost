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

    pub fn generate_forest_chunk(&mut self, pid: Stable<PlaneId>, cpos: V2) -> GenChunk {
        let seed: (u32, u32, u32, u32) = self.rng.gen();
        let mut rng: XorShiftRng = SeedableRng::from_seed([seed.0, seed.1, seed.2, seed.3]);

        let mut grid = DscGrid::new(scalar(48), 4, |_pos, level, _phase| {
            match level {
                3 => 2,
                2 => 1,
                _ => 0,
            }
        });


        let mut loaded_dirs = 0;
        // Load values from adjacent pregenerated chunks.
        for (i, &dir) in DIRS.iter().enumerate() {
            match self.summary.load(pid, cpos + dir) {
                Ok(_) => {},
                Err(_) => continue,
            };
            loaded_dirs |= 1 << i;
            let summ = self.summary.get(pid, cpos + dir);

            let base = (dir + scalar(1)) * scalar(CHUNK_SIZE);
            let bounds = Region::new(base, base + scalar(CHUNK_SIZE + 1));
            for pos in bounds.points() {
                let val = summ.ds_levels[bounds.index(pos)];
                grid.set_value(pos, val);
            }
        }
        debug!("generate {:x} {:?}: seed {:?}, loaded {:x}",
               pid.unwrap(), cpos, seed, loaded_dirs);

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
