use rand::Rng;

use libserver_types::*;

use StdRng;
use algo::disk_sampler::DiskSampler;
use algo::triangulate;
use prop::GlobalProperty;

use super::{DUNGEON_SIZE, ENTRANCE_POS};
use super::summary::PlaneSummary;


pub struct Plan {
    rng: StdRng,
}

impl Plan {
    pub fn new(rng: StdRng) -> Plan {
        Plan {
            rng: rng,
        }
    }
}

pub struct Temporary {
    vert_samp: DiskSampler,
    vertices: Vec<V2>,
    idx_edges: Vec<(u16, u16)>,

    // TODO:
    // edges: Vec<(V2, V2)>,
    // vaults: Vec<_>,
}

const PADDING: i32 = 32;

impl GlobalProperty for Plan {
    type Summary = PlaneSummary;
    type Temporary = Temporary;

    fn init(&mut self, _: &PlaneSummary) -> Temporary {
        // We want a DUNGEON_SIZE x DUNGEON_SIZE region, but to keep things uniform around the
        // edges, we fill a larger region with vertices and edges, then truncate to the desired
        // size.
        let vert_samp = DiskSampler::new(scalar(DUNGEON_SIZE + 2 * PADDING), 16, 32);

        Temporary {
            vert_samp: vert_samp,
            vertices: Vec::new(),
            idx_edges: Vec::new(),
        }
    }

    fn generate(&mut self, tmp: &mut Temporary) {
        tmp.vert_samp.add_init_point(ENTRANCE_POS + scalar(PADDING));
        tmp.vert_samp.generate(&mut self.rng, 30);

        let base = scalar(PADDING);
        tmp.vertices = tmp.vert_samp.points().iter()
                                    .map(|&p| p - base)
                                    .collect();

        tmp.idx_edges = triangulate::triangulate(&tmp.vertices)
                            .into_iter()
                            .filter(|_| self.rng.gen_range(0, 2) < 1)
                            .collect();
    }

    fn save(&mut self, tmp: &Temporary, summ: &mut PlaneSummary) {
        summ.vertices = tmp.vertices.clone();
        summ.edges = tmp.idx_edges.clone();
    }
}
