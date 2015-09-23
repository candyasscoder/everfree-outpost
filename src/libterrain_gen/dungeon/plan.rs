use std::collections::{HashMap, HashSet};
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
    base_verts: Vec<V2>,
    vert_level: Vec<u8>,
    conn_map: HashMap<u16, Vec<u16>>,

    edges: Vec<(V2, V2)>,
    vaults: Vec<Box<Vault>>,
}

impl Temporary {
    fn emit_edge(&mut self, i: u16, j: u16) {
        self.edges.push((self.base_verts[i as usize], self.base_verts[j as usize]));
    }
}

const PADDING: i32 = 32;

impl<'d> GlobalProperty for Plan<'d> {
    type Summary = PlaneSummary;
    type Temporary = Temporary;
    type Result = ();

    fn init(&mut self, _: &PlaneSummary) -> Temporary {
        Temporary {
            base_verts: Vec::new(),
            vert_level: Vec::new(),
            conn_map: HashMap::new(),
            edges: Vec::new(),
            vaults: Vec::new(),
        }
    }

    fn generate(&mut self, tmp: &mut Temporary) {
        // We want a DUNGEON_SIZE x DUNGEON_SIZE region, but to keep things uniform around the
        // edges, we fill a larger region with vertices and edges, then truncate to the desired
        // size.
        let mut vert_samp = DiskSampler::new(scalar(DUNGEON_SIZE + 2 * PADDING), 16, 32);
        vert_samp.add_init_point(ENTRANCE_POS + scalar(PADDING));
        vert_samp.generate(&mut self.rng, 30);

        let base = scalar(PADDING);
        tmp.base_verts = vert_samp.points().iter()
                                  .map(|&p| p - base)
                                  .collect::<Vec<_>>();
        drop(vert_samp);
        tmp.vert_level = iter::repeat(0).take(tmp.base_verts.len()).collect();
        tmp.conn_map = HashMap::with_capacity(tmp.base_verts.len());

        // Use unfiltered verts for `triangulate` to avoid long, skinny triangles near the edges of
        // the map.
        let mut edges = triangulate::triangulate(&tmp.base_verts);
        let bounds = Region::new(scalar(0), scalar(DUNGEON_SIZE));
        for &(i, j) in &edges {
            if !bounds.contains(tmp.base_verts[i as usize]) ||
               !bounds.contains(tmp.base_verts[j as usize]) {
                continue;
            }
            tmp.conn_map.entry(i).or_insert_with(|| Vec::new()).push(j);
            tmp.conn_map.entry(j).or_insert_with(|| Vec::new()).push(i);
        }
        drop(edges);

        self.gen_paths(tmp);
    }

    fn save(&mut self, tmp: Temporary, summ: &mut PlaneSummary) {
        summ.edges = tmp.edges;
        summ.vaults = tmp.vaults;
    }
}

impl<'d> Plan<'d> {
    fn gen_paths(&mut self, tmp: &mut Temporary) {
        let origin = tmp.base_verts.iter().position(|&p| p == ENTRANCE_POS)
                        .expect("ENTRANCE_POS should always be in base_verts") as u16;


        let mut heads = {
            let neighbors = tmp.conn_map.get_mut(&origin).unwrap();
            self.rng.shuffle(neighbors);
            [neighbors[0],
             neighbors[1],
             neighbors[2]]
        };

        tmp.vert_level[origin as usize] = 1;
        for &v in &heads {
            tmp.vert_level[origin as usize] = 2;
            tmp.emit_edge(origin, v);
        }

        for i in 0 .. 30 {
            let j = self.rng.gen_range(0, heads.len());
            let v1 = heads[j];
            let v2 = {
                let get_iter = || tmp.conn_map[&v1].iter().map(|&v| v)
                                     .filter(|&v| tmp.vert_level[v as usize] == 0);
                let count = get_iter().count();
                let idx = self.rng.gen_range(0, count);
                if let Some(v) = get_iter().nth(idx) {
                    v
                } else {
                    continue;
                }
            };

            tmp.emit_edge(v1, v2);
            tmp.vert_level[v2 as usize] = i + 3;
            heads[j] = v2;
        }
    }
}
