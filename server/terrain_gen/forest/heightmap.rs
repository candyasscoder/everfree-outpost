use rand::Rng;

use physics::{CHUNK_BITS, CHUNK_SIZE};
use types::*;

use terrain_gen::StdRng;
use terrain_gen::dsc::{DscGrid, Phase};
use terrain_gen::prop::LocalProperty;

use super::summary::ChunkSummary;
use super::{power, exp_power};


pub struct Heightmap {
    rng: StdRng,
    super_heightmap: [u8; 4 * 4],
}

impl Heightmap {
    pub fn new<F>(cpos: V2, rng: StdRng, mut f: F) -> Heightmap
            where F: FnMut(V2) -> u8 {
        let mut g = |x, y| f(V2::new(x, y) + cpos - scalar(1));
        Heightmap {
            rng: rng,
            super_heightmap:
                [g(0, 0), g(1, 0), g(2, 0), g(3, 0),
                 g(0, 1), g(1, 1), g(2, 1), g(3, 1),
                 g(0, 2), g(1, 2), g(2, 2), g(3, 2),
                 g(0, 3), g(1, 3), g(2, 3), g(3, 3)],
        }
    }
}

impl LocalProperty for Heightmap {
    type Summary = ChunkSummary;
    type Temporary = DscGrid;

    fn init(&mut self) -> DscGrid {
        let mut grid = DscGrid::new(scalar(CHUNK_SIZE * 3), CHUNK_BITS as u8);

        let super_bounds = Region::<V2>::new(scalar(0), scalar(4));
        for offset in super_bounds.points() {
            let level = self.super_heightmap[super_bounds.index(offset)];
            let pos = offset * scalar(CHUNK_SIZE);
            grid.set_range(pos, level - 1, level);
        }

        grid
    }

    fn load(&mut self, grid: &mut DscGrid, dir: V2, summ: &ChunkSummary) {
        let base = (dir + scalar(1)) * scalar(CHUNK_SIZE);
        let bounds = Region::new(base,
                                 base + scalar(CHUNK_SIZE + 1));
        for pos in bounds.points() {
            let val = summ.heightmap[bounds.index(pos)];
            grid.set_range(pos, val, val);
        }

        // Set "constrained" flag for all edges/corners shared with the center chunk.
        let center_bounds = Region::new(scalar(CHUNK_SIZE),
                                        scalar(CHUNK_SIZE * 2 + 1));
        for pos in bounds.intersect(center_bounds).points() {
            grid.set_constrained(pos);
        }
    }

    fn generate(&mut self, grid: &mut DscGrid) {
        grid.fill(&mut self.rng,
                  |_offset, level, phase| {
                      if level == 3 && phase == Phase::Square { 1 } else { 0 }
                  });
    }

    fn save(&mut self, grid: &DscGrid, summ: &mut ChunkSummary) {
        let base = scalar(CHUNK_SIZE);
        let bounds = Region::new(base,
                                 base + scalar(CHUNK_SIZE + 1));
        for pos in bounds.points() {
            let val = grid.get_value(pos).unwrap();
            summ.heightmap[bounds.index(pos)] = val;
        }
    }
}
