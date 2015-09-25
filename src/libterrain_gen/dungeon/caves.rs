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

use super::{DUNGEON_SIZE, ENTRANCE_POS};
use super::summary::ChunkSummary;
use super::summary::PlaneSummary;
use super::vault::Vault;


pub struct Caves<'a> {
    rng: StdRng,
    cpos: V2,
    plane_summ: &'a PlaneSummary,
    vaults: &'a [&'a Vault],
}

impl<'a> Caves<'a> {
    pub fn new(rng: StdRng,
               cpos: V2,
               plane_summ: &'a PlaneSummary,
               vaults: &'a [&'a Vault]) -> Caves<'a> {
        Caves {
            rng: rng,
            cpos: cpos,
            plane_summ: plane_summ,
            vaults: vaults,
        }
    }
}

impl<'a> LocalProperty for Caves<'a> {
    type Summary = ChunkSummary;
    type Temporary = CellularGrid;
    type Result = ();

    fn init(&mut self, summ: &ChunkSummary) -> CellularGrid {
        let mut grid = CellularGrid::new(scalar(CHUNK_SIZE * 3 + 1));

        for pos in grid.bounds().points() {
            let is_wall = self.rng.gen_range(0, 10) < 7;
            //let is_wall = true;
            grid.set(pos, is_wall);
        }

        let base = self.cpos * scalar(CHUNK_SIZE) - scalar(CHUNK_SIZE);
        let mut blob = BlobGrid::new(scalar(CHUNK_SIZE * 3 + 1));

        let bounds = grid.bounds() + base;
        for &(a, b) in &self.plane_summ.edges {
            if !bounds.contains(a) && !bounds.contains(b) {
                continue;
            }
            blob.clear();
            algo::line_points(a, b, |pos, _| {
                let pos = pos - base;
                for offset in Region::new(scalar(0), scalar(1)).points_inclusive() {
                    if grid.bounds().contains(pos + offset) {
                        blob.add_point(pos + offset);
                        grid.set_fixed(pos + offset, false);
                    }
                }
            });
            let len = (b - a).abs().max();
            blob.expand_with_callback(&mut self.rng, len as usize * 5, |pos| {
                grid.set(pos, false);
            });
        }

        if bounds.contains(ENTRANCE_POS) {
            for offset in Region::new(scalar(-1), scalar(5)).points() {
                let p = ENTRANCE_POS + offset - base;
                if grid.bounds().contains(p) {
                    grid.set_fixed(p, false);
                }
            }
        }

        for v in self.vaults {
            v.gen_cave_grid(&mut grid, bounds);
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

    fn save(&mut self, grid: CellularGrid, summ: &mut ChunkSummary) {
        let base: V2 = scalar(CHUNK_SIZE);
        let bounds = Region::new(base, base + scalar(CHUNK_SIZE + 1));
        let cave_walls = summ.cave_walls_mut();

        for pos in bounds.points() {
            cave_walls.set(bounds.index(pos), grid.get(pos));
        }
    }
}
