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

        for pos in grid.bounds().points() {
            let is_wall = self.rng.gen_range(0, 10) < 6;
            grid.set(pos, is_wall);
        }

        let base = self.cpos * scalar(CHUNK_SIZE) - scalar(CHUNK_SIZE);
        {
            let mut mark_square = |pos| {
                for &(ox, oy) in &[(0, 0), (0, 1), (1, 1), (1, 0)] {
                    let pos = pos + V2::new(ox, oy);
                    if grid.bounds().contains(pos - base) {
                        grid.set_fixed(pos - base, false);
                    }
                }
            };

            for &pos in &self.plane_summ.vertices {
                mark_square(pos);
            }

            for &(i, j) in &self.plane_summ.edges {
                let a = self.plane_summ.vertices[i as usize];
                let b = self.plane_summ.vertices[j as usize];
                const STEPS: i32 = 32;
                for d in 0 .. STEPS + 1 {
                    let pos = a + (b - a) * scalar(d) / scalar(STEPS);
                    mark_square(pos);
                }
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
