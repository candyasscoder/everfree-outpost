use std::cmp;
use rand::Rng;

use libphysics::CHUNK_SIZE;
use libserver_types::*;

use StdRng;
use algo::cellular::CellularGrid;
use algo::dsc::DscGrid;
use prop::LocalProperty;

use super::summary::ChunkSummary;
use super::summary::PlaneSummary;


pub struct Caves<'a> {
    rng: StdRng,
    cpos: V2,
    plane_summ: &'a PlaneSummary,
}

impl<'a> Caves<'a> {
    pub fn new(rng: StdRng,
               cpos: V2,
               plane_summ: &'a PlaneSummary) -> Caves<'a> {
        Caves {
            rng: rng,
            cpos: cpos,
            plane_summ: plane_summ,
        }
    }
}

impl<'a> LocalProperty for Caves<'a> {
    type Summary = ChunkSummary;
    type Temporary = CellularGrid;

    fn init(&mut self, summ: &ChunkSummary) -> CellularGrid {
        let mut grid = CellularGrid::new(scalar(CHUNK_SIZE * 3 + 1));

        /*
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
            let entrance_weight = summ.cave_entrances.iter()
                                      .filter(|&e| e.z == self.layer as i32 * 2)
                                      .map(|&e| (rel_pos - e.reduce()).abs().max())
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
        */

        for pos in grid.bounds().points() {
            let is_wall = self.rng.gen_range(0, 10) < 5;
            grid.set(pos, is_wall);
        }

        let base = self.cpos * scalar(CHUNK_SIZE) - scalar(CHUNK_SIZE);
        for &pos in &self.plane_summ.vertices {
            if grid.bounds().contains(pos - base) {
                grid.set_fixed(pos - base, false);
            }
        }

        grid
    }

    fn load(&mut self, grid: &mut CellularGrid, dir: V2, summ: &ChunkSummary) {
        let base = (dir + scalar(1)) * scalar(CHUNK_SIZE);
        let bounds = Region::new(base, base + scalar(CHUNK_SIZE + 1));
        let cave_walls = summ.cave_walls();

        for pos in bounds.points() {
            grid.set_fixed(pos, cave_walls.get(bounds.index(pos)));
        }
    }

    fn generate(&mut self, grid: &mut CellularGrid) {
        for _ in 0 .. 5 {
            grid.step(|here, active, total| 2 * (here as u8 + active) > total);
        }
    }

    fn save(&mut self, grid: &CellularGrid, summ: &mut ChunkSummary) {
        let base: V2 = scalar(CHUNK_SIZE);
        let bounds = Region::new(base, base + scalar(CHUNK_SIZE + 1));
        let cave_walls = summ.cave_walls_mut();

        for pos in bounds.points() {
            cave_walls.set(bounds.index(pos), grid.get(pos));
        }
    }
}
