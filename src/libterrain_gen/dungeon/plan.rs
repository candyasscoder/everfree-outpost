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
use super::vault::Door;
use super::vault::Entrance;


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
    fn emit_edge(&mut self, i: u16, j: u16) -> usize {
        self.edges.push((self.base_verts[i as usize], self.base_verts[j as usize]));
        self.edges.len() - 1
    }

    fn choose_neighbor<F>(&self, rng: &mut StdRng, v0: u16, f: F) -> Option<u16>
            where F: Fn(&Temporary, u16) -> bool {
        let get_iter = || self.conn_map[&v0].iter().map(|&v| v)
                              .filter(|&v| f(self, v));
        let count = get_iter().count();
        if count == 0 {
            return None;
        }

        let idx = rng.gen_range(0, count);
        Some(get_iter().nth(idx).unwrap())
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
        let mut vert_samp = DiskSampler::new(scalar(DUNGEON_SIZE + 2 * PADDING), 24, 48);
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

        tmp.vaults.push(Box::new(Entrance::new(ENTRANCE_POS)));
        self.gen_paths(tmp);
    }

    fn save(&mut self, tmp: Temporary, summ: &mut PlaneSummary) {
        summ.edges = tmp.edges;
        summ.vaults = tmp.vaults;
    }
}

#[derive(Clone)]
struct Path {
    cur_vert: u16,
    last_pos: V2,
    level: u8,
    last_edge: usize,
}

impl Path {
    fn new(cur_vert: u16, last_pos: V2) -> Path {
        Path {
            cur_vert: cur_vert,
            last_pos: last_pos,
            level: 1,
            last_edge: -1_isize as usize,
        }
    }

    fn gen_tunnel(&mut self, ctx: &mut Plan, tmp: &mut Temporary) -> bool {
        let opt_vert = tmp.choose_neighbor(&mut ctx.rng,
                                           self.cur_vert,
                                           |tmp, v| tmp.vert_level[v as usize] == 0);
        let vert = match opt_vert {
            Some(v) => v,
            None => return false,
        };

        self.last_pos = tmp.base_verts[self.cur_vert as usize];
        self.last_edge = tmp.emit_edge(self.cur_vert, vert);
        tmp.vert_level[vert as usize] = self.level;
        self.cur_vert = vert;
        true
    }

    fn gen_door(&mut self, ctx: &mut Plan, tmp: &mut Temporary) -> bool {
        let center = tmp.base_verts[self.cur_vert as usize];

        fn assign_corner(dir: V2) -> Option<i32> {
            if dir.y >= 3 {
                // Connect directly to the entrance/exit point.
                Some(0)
            } else if dir.x.abs() >= 5 {
                // Connect to the corner. then to the entrance/exit point.
                Some(dir.x.signum())
            } else {
                // Would need to connect through (at least) two corners.  Too complicated; just
                // give up.
                None
            }
        }

        let entrance = self.last_pos;
        let entrance_dir = entrance - center;
        let entrance_corner = match assign_corner(entrance_dir) {
            Some(c) => c,
            None => return false,
        };

        let opt_exit_vert = tmp.choose_neighbor(&mut ctx.rng, self.cur_vert, |tmp, v| {
            if tmp.vert_level[v as usize] != 0 {
                return false;
            }

            let exit = tmp.base_verts[v as usize];
            let exit_dir = exit - center;
            let exit_corner = match assign_corner(exit_dir * V2::new(1, -1)) {
                Some(c) => c,
                None => return false,
            };
            if exit_corner == entrance_corner &&
               exit_dir.y > entrance_dir.y {
                // The entrance and exit lines would cross over one another.
                return false
            }

            true
        });
        let exit_vert = match opt_exit_vert {
            Some(v) => v,
            None => return false,
        };
        let exit = tmp.base_verts[exit_vert as usize];
        let exit_dir = exit - center;
        // If the corner assignment is not possible, the vertex should have been filtered out.
        let exit_corner = assign_corner(exit_dir * V2::new(1, -1)).unwrap();

        let entrance_target_pos = center + V2::new(0, 3);
        let exit_target_pos = center + V2::new(0, -3);

        if entrance_corner == 0 {
            tmp.edges[self.last_edge].1 = entrance_target_pos;
        } else {
            let entrance_corner_pos = center + V2::new(5 * entrance_corner, 4);
            tmp.edges[self.last_edge].1 = entrance_corner_pos;
            tmp.edges.push((entrance_corner_pos, entrance_target_pos));
        }

        if exit_corner == 0 {
            tmp.edges.push((exit_target_pos, exit))
        } else {
            let exit_corner_pos = center + V2::new(5 * exit_corner, -4);
            tmp.edges.push((exit_target_pos, exit_corner_pos));
            tmp.edges.push((exit_corner_pos, exit));
        }

        self.last_pos = center;
        self.last_edge = tmp.edges.len() - 1;
        self.level += 1;
        tmp.vert_level[exit_vert as usize] = self.level;
        self.cur_vert = exit_vert;

        tmp.vaults.push(Box::new(Door::new(center, (entrance_corner as i8, exit_corner as i8))));

        true
    }
}

impl<'d> Plan<'d> {
    fn gen_paths(&mut self, tmp: &mut Temporary) {
        let origin = tmp.base_verts.iter().position(|&p| p == ENTRANCE_POS)
                        .expect("ENTRANCE_POS should always be in base_verts") as u16;

        let mut paths = Vec::new();
        self.rng.shuffle(tmp.conn_map.get_mut(&origin).unwrap());
        for &v in &tmp.conn_map[&origin][0..3] {
            paths.push(Path::new(v, ENTRANCE_POS));
        }

        tmp.vert_level[origin as usize] = 1;
        for p in &mut paths {
            tmp.vert_level[p.cur_vert as usize] = 1;
            p.last_edge = tmp.emit_edge(origin, p.cur_vert);
        }

        while paths.len() > 0 {
            let j = self.rng.gen_range(0, paths.len());

            // 1) Sometimes try to generate a door.
            if self.rng.gen_range(0, 10) < 5 && paths[j].gen_door(self, tmp) {
                // Successfully generated a door
                continue;
            }

            // 2) Generate a tunnel and maybe some forks.
            while self.rng.gen_range(0, 10) < 3 {
                // Generate a fork
                let mut new_path = paths[j].clone();
                if new_path.gen_tunnel(self, tmp) {
                    paths.push(new_path);
                }
            }

            if paths[j].gen_tunnel(self, tmp) {
                // Successfully generated a tunnel.
                continue;
            }

            // Nothing was successfully generated.
            paths.swap_remove(j);
        }
    }
}
