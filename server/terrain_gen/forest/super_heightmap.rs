use rand::Rng;

use types::*;

use terrain_gen::StdRng;
use terrain_gen::dsc::DscGrid;
use terrain_gen::prop::LocalProperty;

use super::summary::{SuperchunkSummary, SUPERCHUNK_BITS, SUPERCHUNK_SIZE};
use super::{power, exp_power};


pub struct SuperHeightmap {
    scpos: V2,
    rng: StdRng,
}

impl SuperHeightmap {
    pub fn new(scpos: V2, rng: StdRng) -> SuperHeightmap {
        SuperHeightmap {
            scpos: scpos,
            rng: rng,
        }
    }
}

impl LocalProperty for SuperHeightmap {
    type Summary = SuperchunkSummary;
    type Temporary = DscGrid;

    fn init(&mut self) -> DscGrid {
        let mut grid = DscGrid::new(scalar(SUPERCHUNK_SIZE * 3), SUPERCHUNK_BITS as u8);

        for step in Region::<V2>::new(scalar(0), scalar(4)).points() {
            let pos = step * scalar(SUPERCHUNK_SIZE);
            let cpos = (self.scpos - scalar(1)) * scalar(SUPERCHUNK_SIZE) + pos;
            let pow = power(&mut self.rng, cpos);
            grid.set_range(pos, 98, 99 + pow / 2);
        }

        grid
    }

    fn load(&mut self, grid: &mut DscGrid, dir: V2, summ: &SuperchunkSummary) {
        let base = (dir + scalar(1)) * scalar(SUPERCHUNK_SIZE);
        let bounds = Region::new(base,
                                 base + scalar(SUPERCHUNK_SIZE + 1));
        for pos in bounds.points() {
            let val = summ.ds_levels[bounds.index(pos)];
            grid.set_range(pos, val, val);
        }

        // Set "constrained" flag for all edges/corners shared with the center chunk.
        let center_bounds = Region::new(scalar(SUPERCHUNK_SIZE),
                                        scalar(SUPERCHUNK_SIZE * 2 + 1));
        for pos in bounds.intersect(center_bounds).points() {
            grid.set_constrained(pos);
        }
    }

    fn generate(&mut self, grid: &mut DscGrid) {
        // Chunk coordinate of the grid's (0, 0).
        let base = (self.scpos - scalar(1)) * scalar(SUPERCHUNK_SIZE);
        let mut rng2 = self.rng.gen::<StdRng>();

        grid.fill(&mut self.rng,
                  |offset, level, _phase| {
                      let ep = exp_power(&mut rng2, base + offset);
                      if 4 - level <= ep { 2 } else { 1 }
                  });
    }

    fn save(&mut self, grid: &DscGrid, summ: &mut SuperchunkSummary) {
        let base = scalar(SUPERCHUNK_SIZE);
        let bounds = Region::new(base,
                                 base + scalar(SUPERCHUNK_SIZE + 1));
        for pos in bounds.points() {
            let val = grid.get_value(pos).unwrap();
            summ.ds_levels[bounds.index(pos)] = val;
        }
    }
}
