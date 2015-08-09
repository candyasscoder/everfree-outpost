use std::cmp;
use rand::Rng;

use physics::CHUNK_SIZE;
use types::*;

use terrain_gen::StdRng;
use terrain_gen::disk_sampler2::DiskSampler;
use terrain_gen::prop::LocalProperty;

use super::summary::ChunkSummary;


pub struct Trees {
    rng: StdRng,
}

impl Trees {
    pub fn new(rng: StdRng) -> Trees {
        Trees {
            rng: rng,
        }
    }
}

impl LocalProperty for Trees {
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
            if bounds.contains(pos) {
                summ.tree_offsets.push(pos - bounds.min);
            }
        }
    }
}
