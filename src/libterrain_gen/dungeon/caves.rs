use std::cmp;
use rand::Rng;

use libphysics::CHUNK_SIZE;
use libserver_types::*;

use StdRng;
use algo;
use algo::blob::BlobGrid;
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
            let is_wall = self.rng.gen_range(0, 10) < 7;
            grid.set(pos, is_wall);
        }

        let base = self.cpos * scalar(CHUNK_SIZE) - scalar(CHUNK_SIZE);
        let mut blob = BlobGrid::new(scalar(CHUNK_SIZE * 3 + 1));

        let bounds = grid.bounds() + base;
        for &(i, j) in &self.plane_summ.edges {
            let a = self.plane_summ.vertices[i as usize];
            let b = self.plane_summ.vertices[j as usize];
            if !bounds.contains(a) && !bounds.contains(b) {
                continue;
            }
            blob.clear();
            algo::line_points(a, b, |pos| {
                let pos = pos - base;
                blob.add_point(pos);
                if grid.bounds().contains(pos) {
                    grid.set_fixed(pos, false);
                }
            });
            let len = (b - a).abs().max();
            blob.expand_with_callback(&mut self.rng, len as usize * 5, |pos| {
                grid.set(pos, false);
            });
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
