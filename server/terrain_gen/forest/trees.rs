use std::cmp;
use rand::Rng;

use physics::CHUNK_SIZE;
use types::*;

use terrain_gen::StdRng;
use terrain_gen::disk_sampler2::DiskSampler;
use terrain_gen::dsc::DscGrid;
use terrain_gen::prop::LocalProperty;

use super::summary::ChunkSummary;


pub struct Trees<'a> {
    rng: StdRng,
    height_grid: &'a DscGrid,
}

impl<'a> Trees<'a> {
    pub fn new(rng: StdRng, height_grid: &'a DscGrid) -> Trees<'a> {
        Trees {
            rng: rng,
            height_grid: height_grid,
        }
    }

    // NB: `pos` is a grid position, in the range 0 .. 3 * CHUNK_SIZE.
    fn check_placement(&self, pos: V2) -> bool {
        // TODO: hardcoded size of "tree" template
        let footprint = Region::new(scalar(0), V2::new(4, 2));
        let target_height = self.height_grid.get_value(pos).unwrap();
        footprint.points_inclusive()
            .all(|offset| {
                let val = self.height_grid.get_value(pos + offset).unwrap();
                val.saturating_sub(98) / 2 == target_height.saturating_sub(98) / 2
            })
    }

    fn add_placement_constraints(&self, pos: V2, summ: &mut ChunkSummary) {
        let target_height = self.height_grid.get_value(pos).unwrap() / 2 * 2;
        let low = target_height;
        let high = target_height + 1;

        let footprint = Region::new(scalar(0), V2::new(4, 2));
        let center_bounds = Region::new(scalar(CHUNK_SIZE),
                                        scalar(CHUNK_SIZE * 2 + 1));

        for offset in footprint.points_inclusive() {
            if center_bounds.contains(offset) {
                continue;
            }
            summ.heightmap_constraints.push((pos + offset, (low, high)));
        }
    }
}

impl<'a> LocalProperty for Trees<'a> {
    type Summary = ChunkSummary;
    type Temporary = DiskSampler;

    fn init(&mut self) -> DiskSampler {
        // min spacing == max spacing == 4 tiles.
        DiskSampler::new(scalar(CHUNK_SIZE * 3), 4, 8)
    }

    fn load(&mut self, samp: &mut DiskSampler, dir: V2, summ: &ChunkSummary) {
        let base = (dir + scalar(1)) * scalar(CHUNK_SIZE);
        for &pos in &summ.tree_offsets {
            samp.add_init_point(pos + base);
        }
    }

    fn generate(&mut self, samp: &mut DiskSampler) {
        samp.generate(&mut self.rng, 30);
    }

    fn save(&mut self, samp: &DiskSampler, summ: &mut ChunkSummary) {
        let bounds = Region::new(scalar(CHUNK_SIZE),
                                 scalar(CHUNK_SIZE * 2));

        summ.tree_offsets = Vec::new();
        for &pos in samp.points() {
            if !bounds.contains(pos) || !self.check_placement(pos) {
                continue;
            }

            // TODO: hardcoded size of "tree" template
            if !bounds.contains_inclusive(pos + V2::new(4, 2)) {
                // The structure extends beyond the bounds of the chunk.  Add extra constraints so
                // that the neighboring chunk will include appropriate terrain.
                self.add_placement_constraints(pos, summ);
            }

            summ.tree_offsets.push(pos - bounds.min);
        }
    }
}
