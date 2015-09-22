use libphysics::CHUNK_SIZE;
use libserver_types::*;

use StdRng;
use algo::cellular::CellularGrid;
use algo::disk_sampler::DiskSampler;
use prop::LocalProperty;

use super::summary::ChunkSummary;


pub struct Treasure<'a> {
    rng: StdRng,
    layer: u8,
    cave_grid: &'a CellularGrid,
}

impl<'a> Treasure<'a> {
    pub fn new(rng: StdRng, layer: u8, cave_grid: &'a CellularGrid) -> Treasure<'a> {
        Treasure {
            rng: rng,
            layer: layer,
            cave_grid: cave_grid,
        }
    }

    // NB: `pos` is a grid position, in the range 0 .. 3 * CHUNK_SIZE.
    fn check_placement(&self, pos: V2) -> bool {
        [(0, 0), (0, 1), (1, 1), (1, 0)].iter().map(|&(x, y)| V2::new(x, y))
            .all(|offset| self.cave_grid.get(pos + offset) == false)
    }
}

impl<'a> LocalProperty for Treasure<'a> {
    type Summary = ChunkSummary;
    type Temporary = DiskSampler;
    type Result = ();

    fn init(&mut self, _: &ChunkSummary) -> DiskSampler {
        // All treasure so far is 1 tile in size.
        // Note that we currently can't set min_spacing to 1, because 1 / sqrt(2) == 0 (in other
        // words, the grid resolution is not high enough to handle it).
        DiskSampler::new(scalar(CHUNK_SIZE * 3), 2, 6)
    }

    fn load(&mut self, samp: &mut DiskSampler, dir: V2, summ: &ChunkSummary) {
        let base = (dir + scalar(1)) * scalar(CHUNK_SIZE);
        for &pos in &summ.treasure_offsets[self.layer as usize] {
            samp.add_init_point(pos + base);
        }
    }

    fn generate(&mut self, samp: &mut DiskSampler) {
        samp.generate(&mut self.rng, 30);
    }

    fn save(&mut self, samp: DiskSampler, summ: &mut ChunkSummary) {
        let bounds = Region::new(scalar(CHUNK_SIZE),
                                 scalar(CHUNK_SIZE * 2));

        let mut offsets = Vec::new();
        for &pos in samp.points() {
            if bounds.contains(pos) && self.check_placement(pos) {
                offsets.push(pos - bounds.min);
            }
        }
        summ.treasure_offsets[self.layer as usize] = offsets;
    }
}
