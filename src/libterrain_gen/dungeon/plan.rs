use std::collections::{BinaryHeap, HashMap, HashSet};
use std::iter;
use std::mem;
use rand::Rng;

use libserver_types::*;
use libserver_config::Data;
use libserver_util::SmallVec;

use StdRng;
use algo::disk_sampler::DiskSampler;
use algo::triangulate;
use algo::union_find::UnionFind;
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

    fn a_star<VertPos, VertWeight>(&self,
                                   source: u16,
                                   target: u16,
                                   vert_pos: VertPos,
                                   mut vert_weight: VertWeight) -> Vec<u16>
            where VertPos: Fn(&T) -> V2,
                  VertWeight: FnMut(u16, &T) -> i32 {
        let dist = |s, t| (vert_pos(self.vert(s)) - vert_pos(self.vert(t))).abs().max();
        let mut weight = |v| vert_weight(v, self.vert(v));

        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        struct Entry {
            cost: i32,
            dist: i32,
            vert: u16,
            parent: u16,
        }
        impl Entry {
            fn new(vert: u16, parent: u16, dist: i32, heur: i32) -> Entry {
                Entry { cost: -(heur + dist), dist: dist, vert: vert, parent: parent }
            }
        }

        let mut q = BinaryHeap::new();
        q.push(Entry::new(source, source, 0, dist(source, target)));

        #[derive(Clone, Copy)]
        struct Record {
            parent: u16,
            closed: bool,
        }
        let mut info = iter::repeat(Record { parent: 0, closed: false })
                           .take(self.verts.len()).collect::<Vec<_>>();
        info[source as usize].parent = source;

        loop {
            let e = match q.pop() {
                Some(x) => x,
                None => { return Vec::new(); }, // No path from source to target
            };
            if info[e.vert as usize].closed {
                // Already saw a better path to this node.
                continue;
            }

            // Found the best path to e.vert, and it goes through e.parent.
            info[e.vert as usize].parent = e.parent;
            info[e.vert as usize].closed = true;

            if e.vert == target {
                // Found the path.
                break;
            }

            // Add neighbors to the queue.
            for &n in self.neighbors(e.vert) {
                if info[n as usize].closed {
                    continue;
                }

                let n_dist = e.dist + dist(e.vert, n) + weight(n);
                let n_heur = dist(n, target);
                q.push(Entry::new(n, e.vert, n_dist, n_heur));
            }
        }

        // Reconstruct the actual path from `parent` links.
        let mut path = Vec::new();
        path.push(target);
        while *path.last().unwrap() != source {
            let last = *path.last().unwrap();
            path.push(info[last as usize].parent);
        }
        path.reverse();
        path
    }
}

struct HighVert {
    pos: V2,
    entrances: SmallVec<V2>,
    exits: SmallVec<V2>,
    //puzzle: Box<Puzzle>,
}

impl HighVert {
    pub fn new(pos: V2) -> HighVert {
        HighVert {
            pos: pos,
            entrances: SmallVec::new(),
            exits: SmallVec::new(),
        }
    }
}

pub struct Temporary {
    high: Graph<HighVert>,
    interstitial_edges: Vec<(V2, V2)>,

    base_verts: Vec<V2>,
    vert_level: Vec<u8>,
    conn_map: HashMap<u16, Vec<u16>>,

    edges: Vec<(V2, V2)>,
    vaults: Vec<Box<Vault>>,
}

const HIGH_SPACING: i32 = 48;
const HALF_HIGH_SPACING: i32 = HIGH_SPACING / 2;
const INTERSTITIAL_SPACING: i32 = 8;

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

        g.add_edge_undir(v1, v2);
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
        let g_origin = g.add_vert(HighVert::new(ENTRANCE_POS));
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

            let new_g_idx = g.add_vert(HighVert::new(*tris.vert(new_idx)));
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

    /// Generate paths through the interstitial space (between puzzle areas).  `self.high` must
    /// already be populated with puzzle area info.
    fn gen_interstitial_paths(&mut self, rng: &mut StdRng) {
        // A new graph, containing closely spaced points in between `self.high.verts`.
        let mut samp = DiskSampler::new(scalar(DUNGEON_SIZE),
                                        INTERSTITIAL_SPACING,
                                        2 * INTERSTITIAL_SPACING);
        for v in &self.high.verts {
            samp.add_init_point(v.pos);
        }
        samp.generate(rng, 30);
        let g = triangulated_graph(samp.points(), samp.bounds());

        let mut used = iter::repeat(false).take(g.verts.len()).collect::<Vec<_>>();
        for s in 0 .. self.high.verts.len() as u16 {
            let g_src = g.verts.iter().position(|&p| p == self.high.vert(s).pos)
                         .expect("all points in `self.high` should be in `g`") as u16;
            // Little bit ugly, but it avoids an undesired borrow.
            for t_idx in 0 .. self.high.neighbors(s).len() {
                let t = self.high.neighbors(s)[t_idx];
                let g_tgt = g.verts.iter().position(|&p| p == self.high.vert(t).pos)
                             .expect("all points in `self.high` should be in `g`") as u16;

                let path = g.a_star(g_src, g_tgt, |x| *x, |v, x| {
                    if used[v as usize] {
                        100000
                    } else {
                        // Add some random noise to the path
                        rng.gen_range(0, 100)
                    }
                });
                if path.len() == 0 {
                    warn!("failed to find a path from {:?} ({}) to {:?} ({})",
                          g.vert(g_src), g_src, g.vert(g_tgt), g_tgt);
                    continue;
                }

                let mut state = 0;
                let mut exit = None;
                let mut entrance = None;
                used[path[0] as usize] = true;
                for i in 1 .. path.len() {
                    used[path[i] as usize] = true;
                    if state == 0 {
                        // Check if this edge moves out of the source region.
                        let delta = self.high.vert(s).pos - *g.vert(path[i]);
                        if delta.mag2() >= HALF_HIGH_SPACING * HALF_HIGH_SPACING {
                            exit = Some(*g.vert(path[i - 1]));
                            state = 1;
                        }
                    }

                    // Not `else if` because we still need to check this on the edge where state
                    // transitions 0 -> 1
                    if state == 1 {
                        self.edges.push((*g.vert(path[i - 1]), *g.vert(path[i])));

                        // Check if this edge moves into the target region.
                        let delta = self.high.vert(t).pos - *g.vert(path[i]);
                        if delta.mag2() < HALF_HIGH_SPACING * HALF_HIGH_SPACING {
                            entrance = Some(*g.vert(path[i]));
                            break
                        }
                    }
                }

                let exit_rel = exit.unwrap() - self.high.vert(s).pos;
                let entrance_rel = entrance.unwrap() - self.high.vert(t).pos;
                self.high.vert_mut(s).exits.push(exit_rel);
                self.high.vert_mut(t).entrances.push(entrance_rel);
            }
        }
    }

    fn gen_maze(&mut self, rng: &mut StdRng, center: V2, entrances: &[V2], exits: &[V2]) {
        let mut samp = DiskSampler::new(scalar(HIGH_SPACING),
                                        INTERSTITIAL_SPACING,
                                        2 * INTERSTITIAL_SPACING);
        for &pos in entrances {
            samp.add_init_point(pos + scalar(HALF_HIGH_SPACING));
        }
        for &pos in exits {
            samp.add_init_point(pos + scalar(HALF_HIGH_SPACING));
        }
        samp.generate(rng, 30);
        let g = triangulated_graph(samp.points(), samp.bounds());

        let mut edges = Vec::new();
        let max_mag2 = HALF_HIGH_SPACING * HALF_HIGH_SPACING;
        for (v1, neighbors) in g.edges.iter().enumerate() {
            let v1 = v1 as u16;
            if (*g.vert(v1) - scalar(HALF_HIGH_SPACING)).mag2() > max_mag2 {
                continue;
            }
            for &v2 in neighbors {
                if (*g.vert(v2) - scalar(HALF_HIGH_SPACING)).mag2() > max_mag2 {
                    continue;
                }
                edges.push((v1, v2));
            }
        }

        let mut uf = UnionFind::new(g.verts.len());
        while edges.len() > 0 {
            let idx = rng.gen_range(0, edges.len());
            let (v1, v2) = edges.swap_remove(idx);
            if !uf.union(v1, v2) {
                continue;
            }

            let pos1 = *g.vert(v1) - scalar(HALF_HIGH_SPACING) + center;
            let pos2 = *g.vert(v2) - scalar(HALF_HIGH_SPACING) + center;
            self.edges.push((pos1, pos2));
        }
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
            interstitial_edges: Vec::new(),
            base_verts: Vec::new(),
            vert_level: Vec::new(),
            conn_map: HashMap::new(),
            edges: Vec::new(),
            vaults: Vec::new(),
        }
    }

    fn generate(&mut self, tmp: &mut Temporary) {
        tmp.gen_high(&mut self.rng);
        tmp.gen_interstitial_paths(&mut self.rng);

        let g = mem::replace(&mut tmp.high, Graph::new());
        for v in &g.verts {
            tmp.gen_maze(&mut self.rng, v.pos, &v.entrances, &v.exits);
        }
        tmp.high = g;
    }

    fn save(&mut self, tmp: Temporary, summ: &mut PlaneSummary) {
        summ.edges = tmp.edges;
        summ.vaults = tmp.vaults;
        summ.verts = tmp.high.verts.iter().map(|v| v.pos).collect();
    }
}
