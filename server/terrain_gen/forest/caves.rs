use std::cmp;
use rand::Rng;

use physics::{CHUNK_BITS, CHUNK_SIZE};
use types::*;

use terrain_gen::StdRng;
use terrain_gen::cellular::CellularGrid;
use terrain_gen::dsc::DscGrid;
use terrain_gen::prop::LocalProperty;

use super::summary::ChunkSummary;
use super::{power, exp_power};


pub struct Caves<'a> {
    rng: StdRng,
    layer: u8,
    level_cutoff: u8,
    heightmap: &'a DscGrid,
    entrances: &'a [V2],
}

impl<'a> Caves<'a> {
    pub fn new(rng: StdRng,
               layer: u8,
               level_cutoff: u8,
               heightmap: &'a DscGrid,
               entrances: &'a [V2]) -> Caves<'a> {
        Caves {
            rng: rng,
            layer: layer,
            level_cutoff: level_cutoff,
            heightmap: heightmap,
            entrances: entrances,
        }
    }
}

impl<'a> LocalProperty for Caves<'a> {
    type Summary = ChunkSummary;
    type Temporary = CellularGrid;

    fn init(&mut self) -> CellularGrid {
        let mut grid = CellularGrid::new(scalar(CHUNK_SIZE * 3 + 1));

        let bounds = grid.bounds();
        for pos in bounds.points() {
            let cur_level = self.heightmap.get_value(pos).unwrap();
            if cur_level < self.level_cutoff {
                // Point is outside the valid region, so mark it as permanent wall.
                grid.set_fixed(pos, true);
                continue;
            }


            let rel_pos = pos - scalar(CHUNK_SIZE);
            // Compute "closeness to entrance" metric.  Value decreases linearly with distance from
            // entrance until eventually reaching zero.
            let entrance_weight = self.entrances.iter()
                                      .map(|&e| (rel_pos - e).abs().max())
                                      .min()
                                      .map_or(0, |x| cmp::max(0, 8 - x));
            let wall_chance = 5 - entrance_weight;
            if wall_chance < 0 {
                // Close enough to the entrance, force open space.
                grid.set_fixed(pos, false);
            } else {
                // Otherwise, `wall_chance` in 10 should be walls.
                let is_wall = self.rng.gen_range(0, 10) < wall_chance;
                grid.set(pos, is_wall);
            }
        }

        grid
    }

    fn load(&mut self, grid: &mut CellularGrid, dir: V2, summ: &ChunkSummary) {
        let base = (dir + scalar(1)) * scalar(CHUNK_SIZE);
        let bounds = Region::new(base, base + scalar(CHUNK_SIZE + 1));
        let layer = summ.cave_wall_layer(self.layer);

        for pos in bounds.points() {
            grid.set_fixed(pos, layer.get(bounds.index(pos)));
        }
    }

    fn generate(&mut self, grid: &mut CellularGrid) {
        for _ in 0 .. 3 {
            grid.step(|here, active, total| 2 * (here as u8 + active) > total);
        }
    }

    fn save(&mut self, grid: &CellularGrid, summ: &mut ChunkSummary) {
        let base: V2 = scalar(CHUNK_SIZE);
        let bounds = Region::new(base, base + scalar(CHUNK_SIZE + 1));
        let layer = summ.cave_wall_layer_mut(self.layer);

        for pos in bounds.points() {
            layer.set(bounds.index(pos), grid.get(pos));
        }
    }
}
