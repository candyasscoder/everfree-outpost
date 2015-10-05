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
use super::vault::{Treasure, TreasureKind, ChestItem};
use super::vault::Library;


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

struct Graph<T> {
    verts: Vec<T>,
    edges: Vec<Vec<u16>>,
    roots: Vec<u16>,
}

impl<T> Graph<T> {
    fn new() -> Graph<T> {
        Graph {
            verts: Vec::new(),
            edges: Vec::new(),
            roots: Vec::new(),
        }
    }

    fn add_vert(&mut self, v: T) -> u16 {
        self.verts.push(v);
        self.edges.push(Vec::new());
        (self.verts.len() - 1) as u16
    }

    fn add_edge(&mut self, a: u16, b: u16) {
        assert!((a as usize) < self.verts.len());
        assert!((b as usize) < self.verts.len());
        self.edges[a as usize].push(b);
    }

    fn add_edge_undir(&mut self, a: u16, b: u16) {
        self.edges[a as usize].push(b);
        self.edges[b as usize].push(a);
    }

    fn neighbors(&self, v: u16) -> &[u16] {
        &self.edges[v as usize]
    }

    fn vert(&self, v: u16) -> &T {
        &self.verts[v as usize]
    }

    fn vert_mut(&mut self, v: u16) -> &mut T {
        &mut self.verts[v as usize]
    }

    fn choose_neighbor<F: Fn(u16) -> bool>(&self, rng: &mut StdRng, v: u16, f: F) -> Option<u16> {
        let mk_iter = || self.neighbors(v).iter().map(|&v| v).filter(|&v| f(v));
        let count = mk_iter().count();
        if count == 0 {
            return None;
        }
        Some(mk_iter().nth(rng.gen_range(0, count)).unwrap())
    }
}

struct HighVert {
    pos: V2,
    //puzzle: Box<Puzzle>,
}

pub struct Temporary {
    high: Graph<HighVert>,

    base_verts: Vec<V2>,
    vert_level: Vec<u8>,
    conn_map: HashMap<u16, Vec<u16>>,

    edges: Vec<(V2, V2)>,
    vaults: Vec<Box<Vault>>,
}

const HIGH_SPACING: i32 = 48;

fn triangulated_graph(raw_verts: &[V2], bounds: Region<V2>) -> Graph<V2> {
    let raw_edges = triangulate::triangulate(raw_verts);

    let mut idx_map = Vec::with_capacity(raw_verts.len());
    let mut g = Graph::new();
    for (i, &p) in raw_verts.iter().enumerate() {
        if bounds.contains(p) {
            let g_idx = g.add_vert(p - bounds.min);
            idx_map.push(g_idx);
        } else {
            idx_map.push(0);
        }
    }

    for (raw_v1, raw_v2) in raw_edges {
        let v1 = idx_map[raw_v1 as usize];
        let v2 = idx_map[raw_v2 as usize];
        if (v1 == 0 && !bounds.contains(raw_verts[raw_v1 as usize])) ||
           (v2 == 0 && !bounds.contains(raw_verts[raw_v2 as usize])) {
            continue;
        }

        g.add_edge(v1, v2);
    }

    g
}

impl Temporary {
    fn gen_high(&mut self, rng: &mut StdRng) {
        let mut samp = DiskSampler::new(scalar(DUNGEON_SIZE + 2 * HIGH_SPACING),
                                        HIGH_SPACING,
                                        2 * HIGH_SPACING);
        samp.add_init_point(ENTRANCE_POS + scalar(HIGH_SPACING));
        samp.generate(rng, 30);

        let bounds = Region::new(scalar(0), scalar(DUNGEON_SIZE)) + scalar(HIGH_SPACING);
        let tris = triangulated_graph(samp.points(), bounds);

        let origin = tris.verts.iter().position(|&p| p == ENTRANCE_POS)
                         .expect("ENTRANCE_POS should always be in samp.points()") as u16;
        let mut used = iter::repeat(false).take(tris.verts.len()).collect::<Vec<_>>();
        let mut g = Graph::new();
        let g_origin = g.add_vert(HighVert { pos: ENTRANCE_POS });
        g.roots.push(g_origin);

        // Populate `g` with a tree, using edges from `tris` and rooted at `origin`.
        #[derive(Clone, Copy)]
        struct Path {
            tris_idx: u16,
            g_idx: u16,
            level: u8,
            remaining: u8,
        }
        let mut paths = Vec::with_capacity(5);
        for _ in 0 .. rng.gen_range(2, 4) {
            paths.push(Path {
                tris_idx: 0,
                g_idx: g_origin,
                level: 0,
                remaining: rng.gen_range(3, 5),
            });
        }
        used[origin as usize] = true;

        while paths.len() > 0 {
            let choice = rng.gen_range(0, paths.len());
            let mut p = paths.swap_remove(choice);

            let new_idx = match tris.choose_neighbor(rng, p.tris_idx, |v| !used[v as usize]) {
                Some(x) => x,
                None => continue,
            };

            let new_g_idx = g.add_vert(HighVert { pos: *tris.vert(new_idx) });
            g.add_edge(p.g_idx, new_g_idx);

            p.tris_idx = new_idx;
            p.level += 1;
            p.remaining -= 1;
            p.g_idx = new_g_idx;
            used[p.tris_idx as usize] = true;

            if p.remaining > 0 {
                paths.push(p);
            }
        }

        self.high = g;
    }

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
            high: Graph::new(),
            base_verts: Vec::new(),
            vert_level: Vec::new(),
            conn_map: HashMap::new(),
            edges: Vec::new(),
            vaults: Vec::new(),
        }
    }

    fn generate(&mut self, tmp: &mut Temporary) {
        tmp.gen_high(&mut self.rng);

        /*
        // We want a DUNGEON_SIZE x DUNGEON_SIZE region, but to keep things uniform around the
        // edges, we fill a larger region with vertices and edges, then truncate to the desired
        // size.
        let mut vert_samp = DiskSampler::new(scalar(DUNGEON_SIZE + 2 * PADDING), 20, 40);
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
        */

        for (v1, neighbors) in tmp.high.edges.iter().enumerate() {
            for &v2 in neighbors {
                let p1 = tmp.high.vert(v1 as u16).pos;
                let p2 = tmp.high.vert(v2).pos;
                tmp.edges.push((p1, p2));
            }
        }
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
                return false;
            }
            if exit_dir.dot(entrance_dir) > 0 {
                // Entrance and exit directions are within 90 degrees of each other.
                return false;
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
            let choice = self.rng.gen_range(0, 100);
            let pos = tmp.base_verts[paths[j].cur_vert as usize];
            if choice < 40 {
                if paths[j].gen_door(self, tmp) {
                    continue;
                }
            } else if choice < 60 {
                let level = paths[j].level;
                // 0.5% chance of hat at level 5, +0.5% for every level thereafter.
                let chance = if level < 5 { 0 } else { level - 5 };
                let (count, item) =
                    if self.rng.gen_range(0, 200) < chance {
                        (1, ChestItem::Hat)
                    } else {
                        // Generate 1-3 keys (avg: 2)
                        (self.rng.gen_range(1, 4), ChestItem::Key)
                    };
                let kind = TreasureKind::Chest(count, item);
                tmp.vaults.push(Box::new(Treasure::new(pos, kind)));
                // Then also make a path
            } else if choice < 63 {
                tmp.vaults.push(Box::new(Treasure::new(pos, TreasureKind::Fountain)));
            } else if choice < 66 {
                tmp.vaults.push(Box::new(Treasure::new(pos, TreasureKind::Trophy)));
            } else if choice < 70 {
                tmp.vaults.push(Box::new(Library::new(pos,
                                                      self.rng.gen_range(3, 8),
                                                      self.rng.gen())));
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
