use std::iter;
use std::{i32, u16};
use rand::Rng;

use libserver_types::*;

use StdRng;
use prop::GlobalProperty;

use super::summary::PlaneSummary;


pub struct GraphEdges {
    rng: StdRng,
}

impl GraphEdges {
    pub fn new(rng: StdRng) -> GraphEdges {
        GraphEdges {
            rng: rng,
        }
    }
}

const NEAREST_COUNT: usize = 5;
const CHOOSE_COUNT: usize = 1;

struct Temporary {
    nearest: Box<[[(u16, i32); NEAREST_COUNT]]>,
    edge_list: Vec<(u16, u16)>,
}

impl GlobalProperty for GraphEdges {
    type Summary = PlaneSummary;
    type Temporary = Temporary;

    fn init(&mut self, summ: &PlaneSummary) -> Temporary {
        let mut nearest = iter::repeat([(u16::MAX, i32::MAX); NEAREST_COUNT])
                .take(summ.vertices.len()).collect::<Vec<_>>().into_boxed_slice();
        info!("generating edges for {} vertices", summ.vertices.len());

        info!("  {:?}", &summ.vertices);

        for (i, &a) in summ.vertices.iter().enumerate() {
            for (j, &b) in summ.vertices.iter().enumerate() {
                let dist2 = (a - b).mag2();

                if dist2 < nearest[i][NEAREST_COUNT - 1].1 {
                    // Start by considering the last slot of the array to be empty.  Then we
                    // compare dist2 against each other element, in reverse order.  If dist2 is
                    // nearer, then shift the current element to the right and keep going.  This
                    // fills the empty space and leaves a new one at the current `k`.  If dist2 is
                    // farther, then drop dist2 in the empty space at `k + 1`.
                    for k in (0 .. NEAREST_COUNT - 1).rev() {
                        if dist2 < nearest[i][k].1 {
                            nearest[i][k + 1] = nearest[i][k];
                            // Special case.  If dist2 is nearer than every slot in the array, drop
                            // it in the first slot at the end.
                            if k == 0 {
                                nearest[i][k] = (j as u16, dist2);
                            }
                        } else {
                            nearest[i][k + 1] = (j as u16, dist2);
                        }
                    }
                }
            }
        }

        Temporary {
            nearest: nearest,
            edge_list: Vec::new(),
        }
    }

    fn generate(&mut self, tmp: &mut Temporary) {
        for i in 0 .. tmp.nearest.len() {
            info!("{}: {:?}", i, tmp.nearest[i]);
            for _ in 0 .. CHOOSE_COUNT {
                let (j, dist) = *self.rng.choose(&tmp.nearest[i]).unwrap();
                if dist == i32::MAX {
                    continue;
                }
                tmp.edge_list.push((i as u16, j));
            }
        }
    }

    fn save(&mut self, tmp: &Temporary, summ: &mut PlaneSummary) {
        //summ.edges = tmp.edge_list.clone();
        summ.edges = ::libterrain_gen_algo::triangulate::triangulate(&summ.vertices);
    }
}
