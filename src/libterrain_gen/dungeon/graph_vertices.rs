use libphysics::CHUNK_SIZE;
use libserver_types::*;

use StdRng;
use algo::disk_sampler::DiskSampler;
use prop::GlobalProperty;

use super::summary::PlaneSummary;


pub struct GraphVertices {
    rng: StdRng,
}

impl GraphVertices {
    pub fn new(rng: StdRng) -> GraphVertices {
        GraphVertices {
            rng: rng,
        }
    }
}

const DUNGEON_SIZE: i32 = 256;
const PADDING: i32 = 32;

impl GlobalProperty for GraphVertices {
    type Summary = PlaneSummary;
    type Temporary = DiskSampler;

    fn init(&mut self, _: &PlaneSummary) -> DiskSampler {
        // We want a DUNGEON_SIZE x DUNGEON_SIZE region, but to keep things uniform around the
        // edges, we fill a larger region with vertices and edges, then truncate to the desired
        // size.
        DiskSampler::new(scalar(DUNGEON_SIZE + 2 * PADDING), 12, 20)
    }

    fn generate(&mut self, samp: &mut DiskSampler) {
        samp.generate(&mut self.rng, 30);
    }

    fn save(&mut self, samp: &DiskSampler, summ: &mut PlaneSummary) {
        let base = scalar(PADDING);
        summ.vertices = samp.points().iter()
                            .map(|&p| p - base)
                            .collect();
    }
}
