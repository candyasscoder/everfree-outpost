use std::iter;
use rand::Rng;

use libserver_types::*;
use libserver_config::Data;

use StdRng;
use algo::disk_sampler::DiskSampler;
use algo::triangulate;
use prop::GlobalProperty;

use super::{DUNGEON_SIZE, ENTRANCE_POS};
use super::summary::PlaneSummary;
use super::vault::Vault;
use super::vault::FloorMarking;


pub struct Plan<'d> {
    rng: StdRng,
    data: &'d Data,
}

impl<'d> Plan<'d> {
    pub fn new(rng: StdRng, data: &'d Data) -> Plan<'d> {
        Plan {
            rng: rng,
            data: data,
        }
    }
}

pub struct Temporary {
    vert_samp: DiskSampler,
    vertices: Vec<V2>,
    idx_edges: Vec<(u16, u16)>,

    vaults: Vec<Box<Vault>>,

    // TODO:
    // edges: Vec<(V2, V2)>,
}

const PADDING: i32 = 32;

impl<'d> GlobalProperty for Plan<'d> {
    type Summary = PlaneSummary;
    type Temporary = Temporary;
    type Result = ();

    fn init(&mut self, _: &PlaneSummary) -> Temporary {
        // We want a DUNGEON_SIZE x DUNGEON_SIZE region, but to keep things uniform around the
        // edges, we fill a larger region with vertices and edges, then truncate to the desired
        // size.
        let vert_samp = DiskSampler::new(scalar(DUNGEON_SIZE + 2 * PADDING), 16, 32);

        Temporary {
            vert_samp: vert_samp,
            vertices: Vec::new(),
            idx_edges: Vec::new(),
            vaults: Vec::new(),
        }
    }

    fn generate(&mut self, tmp: &mut Temporary) {
        tmp.vert_samp.add_init_point(ENTRANCE_POS + scalar(PADDING));
        tmp.vert_samp.generate(&mut self.rng, 30);

        let base = scalar(PADDING);
        tmp.vertices = tmp.vert_samp.points().iter()
                                    .map(|&p| p - base)
                                    .collect();

        let mut count = iter::repeat(0).take(tmp.vertices.len())
                                       .collect::<Vec<_>>().into_boxed_slice();
        let mut edges = triangulate::triangulate(&tmp.vertices);
        self.rng.shuffle(&mut edges);
        for (i, j) in edges.into_iter() {
            if count[i as usize] >= 3 || count[j as usize] >= 3 {
                continue;
            }
            tmp.idx_edges.push((i, j));
            count[i as usize] += 1;
            count[j as usize] += 1;
        }

        let floor_id = self.data.structure_templates.get_id("wood_floor/center/v0");
        for &pos in &tmp.vertices {
            if self.rng.gen_range(0, 10) < 7 {
                continue;
            }

            tmp.vaults.push(Box::new(FloorMarking::new(pos, floor_id)));
        }
        info!("generated {} vaults", tmp.vaults.len());
    }

    fn save(&mut self, tmp: Temporary, summ: &mut PlaneSummary) {
        summ.vertices = tmp.vertices;
        summ.edges = tmp.idx_edges;
        summ.vaults = tmp.vaults;
    }
}
