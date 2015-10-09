use std::collections::{BinaryHeap, HashMap, HashSet};
use std::iter;
use std::mem;
use rand::Rng;

use libserver_types::*;
use libserver_config::Data;
use libserver_util::{SmallVec, SmallSet};

use StdRng;
use algo::disk_sampler::DiskSampler;
use algo::reservoir_sample_weighted;
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

    fn map<U, F: FnMut(T) -> U>(self, mut f: F) -> Graph<U> {
        Graph {
            verts: self.verts.into_iter().map(f).collect(),
            edges: self.edges,
            roots: self.roots,
        }
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
        let mut info = mk_array(Record { parent: 0, closed: false }, self.verts.len());
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

fn mk_array<T: Copy>(init: T, len: usize) -> Box<[T]> {
    iter::repeat(init).take(len).collect::<Vec<_>>().into_boxed_slice()
}

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

fn simple_blob(g: &Graph<BaseVert>, rng: &mut StdRng, start: u16, size: usize) -> Vec<u16> {
    let mut chosen = Vec::new();
    let mut level = mk_array(0, g.verts.len());
    // Keep lists of vertices connected by 1 edge, by 2 edges, and by 3+ edges.
    let mut pending = [HashSet::new(), HashSet::new(), HashSet::new()];
    let mut total_pending = 0;

    const MAX_EDGES: usize = 3;
    const CHOSEN: u8 = 255;

    level[start as usize] = 1;
    pending[0].insert(start);
    total_pending = 1;

    // Stop if there are no more vertices to choose from.  Also stop after choosing a total of
    // `size` vertices, but only once there are no pending 3-edge vertices.
    while total_pending > 0 && (chosen.len() < size || pending[MAX_EDGES - 1].len() > 0) {
        let mut v = None;
        for p in pending.iter_mut().rev() {
            if p.len() == 0 {
                continue;
            }
            let idx = rng.gen_range(0, p.len());
            let choice = p.iter().map(|&x| x).nth(idx).unwrap();
            p.remove(&choice);
            v = Some(choice);
            total_pending -= 1;
            break;
        }
        // Never fails because at least one Vec in `pending` is nonempty.
        let v = v.unwrap();

        chosen.push(v);
        level[v as usize] = CHOSEN;

        for &n in g.neighbors(v) {
            if g.vert(n).area != 0 {
                continue;
            }

            let old_level = level[n as usize];
            if old_level == CHOSEN || old_level >= MAX_EDGES as u8 {
                continue;
            }
            if old_level > 0 {
                pending[old_level as usize - 1].remove(&n);
            } else {
                total_pending += 1;
            }

            let new_level = old_level + 1;
            level[n as usize] = new_level;
            pending[new_level as usize - 1].insert(n);
        }
    }

    chosen
}


struct BaseVert {
    pos: V2,
    area: u32,
    label: u32,
}

impl BaseVert {
    pub fn new(pos: V2) -> BaseVert {
        BaseVert {
            pos: pos,
            area: 0,
            label: 0,
        }
    }
}

pub struct Temporary {
    base: Graph<BaseVert>,
    next_area: u32,

    edges: Vec<(V2, V2)>,
}

const AREA_NONE: u32 = 0;
const AREA_TUNNEL: u32 = 1;
const AREA_FIRST_PUZZLE: u32 = 2;

/// Minimum vertex spacing in the base graph.
const BASE_SPACING: i32 = 12;
/// Amount of extra space to add around the border, to avoid artifacts near the boundaries of the
/// generated graph.
const BASE_PADDING: i32 = 2 * BASE_SPACING;

impl Temporary {
    fn gen_base(&mut self, rng: &mut StdRng) {
        let mut samp = DiskSampler::new(scalar(DUNGEON_SIZE + 2 * BASE_PADDING),
                                        BASE_SPACING,
                                        2 * BASE_SPACING);
        samp.add_init_point(ENTRANCE_POS + scalar(BASE_PADDING));
        samp.generate(rng, 30);

        let bounds = Region::new(scalar(0), scalar(DUNGEON_SIZE)) + scalar(BASE_PADDING);
        self.base = triangulated_graph(samp.points(), bounds).map(BaseVert::new);
    }

    fn alloc_area(&mut self) -> u32 {
        let area = self.next_area;
        self.next_area += 1;
        area
    }

    fn mark_area(&mut self, verts: &[u16]) -> u32 {
        let area = self.alloc_area();

        for &v in verts {
            self.base.vert_mut(v).area = area;
            self.base.vert_mut(v).label = 0;
        }

        area
    }

    fn gen_entrance(&mut self, rng: &mut StdRng) {
        let origin = self.base.verts.iter().position(|v| v.pos == ENTRANCE_POS)
                         .expect("ENTRANCE_POS should always be in self.base.verts") as u16;
        let mut verts = Vec::with_capacity(self.base.neighbors(origin).len() + 1);
        verts.push(origin);
        verts.extend(self.base.neighbors(origin).iter().map(|&x| x));
        let verts = verts.into_boxed_slice();

        let area = self.mark_area(&verts);
        for &v1 in &*verts {
            for &v2 in self.base.neighbors(v1) {
                if self.base.vert(v2).area == area && v1 < v2 {
                    self.edges.push((self.base.vert(v1).pos,
                                     self.base.vert(v2).pos));
                }
            }
        }

        for _ in 0 .. 3 {
            let start = self.base.choose_neighbor(rng, origin,
                                                  |v| self.base.vert(v).label == 0).unwrap();
            self.base.vert_mut(start).label = 1;
            let dir = self.base.vert(start).pos - self.base.vert(origin).pos;
            let end = self.gen_tunnel(rng, start, 3, dir);
            self.gen_door(rng, end);
        }
    }

    fn step_tunnel(&mut self, rng: &mut StdRng, v: u16, dir: V2) -> Option<u16> {
        let v_pos = self.base.vert(v).pos;
        reservoir_sample_weighted(rng,
                self.base.neighbors(v).iter()
                    .filter(|&&n| self.base.vert(n).area == 0)
                    .map(|&n| (n, dir.dot(self.base.vert(n).pos - v_pos))))
    }

    fn gen_tunnel(&mut self, rng: &mut StdRng, v: u16, len: usize, dir: V2) -> u16 {
        let mut cur = v;
        for _ in 0 .. len {
            let next = match self.step_tunnel(rng, cur, dir) {
                Some(x) => x,
                None => break,
            };
            let p1 = self.base.vert(cur).pos;
            let p2 = self.base.vert(next).pos;
            self.edges.push((p1, p2));
            self.base.vert_mut(next).area = AREA_TUNNEL;
            cur = next;
        }
        cur
    }

    fn gen_door(&mut self, rng: &mut StdRng, start: u16) -> u16 {
        let area = self.alloc_area();

        let mut seen = mk_array(false, self.base.verts.len());
        let mut level_verts = vec![start];
        seen[start as usize] = true;

        let mut candidates = Vec::new();
        for _ in 0 .. 2 {
            let mut next_level_verts = Vec::new();
            info!("{} verts in current level", level_verts.len());
            for &v in &level_verts {
                let v_pos = self.base.vert(v).pos;
                for &n in self.base.neighbors(v) {
                    if self.base.vert(n).area != 0 {
                        continue;
                    }

                    // Check if this is a reasonable edge for a door.
                    let n_pos = self.base.vert(n).pos;
                    let dir = n_pos - v_pos;
                    if dir.dot(V2::new(1, -1)) > 0 && dir.dot(V2::new(-1, -1)) > 0 {
                        candidates.push((v, n));
                    }

                    if seen[n as usize] {
                        continue;
                    }
                    seen[n as usize] = true;
                    next_level_verts.push(n);
                }
            }
            level_verts = next_level_verts;
        }

        for (v1, v2) in candidates {
            // Check the vertices to the sides of the door.
            let mut side_verts = SmallVec::new();
            for &n1 in self.base.neighbors(v1) {
                for &n2 in self.base.neighbors(v2) {
                    if n1 == n2 {
                        side_verts.push(n1);
                    }
                }
            }
            if side_verts.iter().any(|&v| self.base.vert(v).area >= AREA_FIRST_PUZZLE) {
                // The door area needs ownership of the triangles on both sides of the
                // entrance-exit edge.
                continue;
            }
            // Note that the search above only produced candidates where the entrance and exit can
            // both be claimed.  (Specifically, the exit is unclaimed, and the entrance is
            // unclaimed or claimed only by AREA_TUNNEL.)

            // Make sure there's a path to the entrance that doesn't cut through the door.
            let path = self.base.a_star(start, v1, |v| v.pos, |v,_| {
                if v == v1 || v == v2 { 10000 } else { 0 }
            });
            if path.len() > 3 {
                continue;
            }

            // Add edges for the extra path.
            let mut last = start;
            for &v in &path[1..] {
                self.edges.push((self.base.vert(last).pos,
                                 self.base.vert(v).pos));
                self.base.vert_mut(v).area = AREA_TUNNEL;
                last = v;
            }

            self.edges.push((self.base.vert(v1).pos,
                             self.base.vert(v2).pos));

            // Take ownership of the necessary vertices.
            self.base.vert_mut(v1).area = area;
            self.base.vert_mut(v2).area = area;
            for &v in side_verts.iter() {
                self.base.vert_mut(v).area = area;
            }

            return v2;
        }

        // Fell through without finding a place to put the door.
        start
    }
}

const PADDING: i32 = 32;

impl<'d> GlobalProperty for Plan<'d> {
    type Summary = PlaneSummary;
    type Temporary = Temporary;
    type Result = ();

    fn init(&mut self, _: &PlaneSummary) -> Temporary {
        Temporary {
            base: Graph::new(),
            next_area: AREA_FIRST_PUZZLE,

            edges: Vec::new(),
        }
    }

    fn generate(&mut self, tmp: &mut Temporary) {
        tmp.gen_base(&mut self.rng);
        tmp.gen_entrance(&mut self.rng);
    }

    fn save(&mut self, tmp: Temporary, summ: &mut PlaneSummary) {
        summ.edges = tmp.edges;
        //summ.vaults = tmp.vaults;
        //summ.verts = tmp.high.verts.iter().map(|v| v.pos).collect();
    }
}
