use std::collections::{BinaryHeap, HashMap, HashSet};
use std::iter::{self, Peekable};
use std::mem;
use std::slice;
use rand::Rng;

use libserver_types::*;
use libserver_config::Data;
use libserver_util::{SmallVec, SmallSet};

use StdRng;
use algo::disk_sampler::DiskSampler;
use algo::{reservoir_sample, reservoir_sample_weighted};
use algo::triangulate;
use algo::union_find::UnionFind;
use prop::GlobalProperty;

use super::{DUNGEON_SIZE, ENTRANCE_POS};
use super::summary::PlaneSummary;
use super::vault::{self, Vault};
use super::types::Triangle;


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

    fn has_edge(&self, v1: u16, v2: u16) -> bool {
        self.edges[v1 as usize].contains(&v2)
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

    fn for_each_triangle<F>(&self, f: F)
            where F: FnMut(u16, u16, u16) {
        self.for_each_triangle_filtered(|_| true, f)
    }

    fn for_each_triangle_filtered<F, Filter>(&self, mut filter: Filter, mut f: F)
            where F: FnMut(u16, u16, u16),
                  Filter: FnMut(u16) -> bool {
        // To avoid duplication, consider only triangles whose vertices are sorted by vertex index.
        for a in 0 .. self.verts.len() as u16 {
            if !filter(a) {
                continue;
            }
            for &b in self.neighbors(a) {
                if b < a || !filter(b) {
                    continue;
                }

                // Find neighbors that `n` and `v` have in common.
                let mut a_ns = self.neighbors(a).iter().map(|&x| x).peekable();
                let mut b_ns = self.neighbors(b).iter().map(|&x| x).peekable();

                // Advance past ineligible vertices (index < n).  Note this whole scheme assumes
                // the neighbor lists are sorted.
                while let Some(&c1) = a_ns.peek() {
                    if c1 < b {
                        a_ns.next();
                    } else {
                        break;
                    }
                }
                while let Some(&c2) = b_ns.peek() {
                    if c2 < b {
                        b_ns.next();
                    } else {
                        break;
                    }
                }

                // Now walk through the two lists like a mergesort, looking for identical elements.
                while let (Some(&c1), Some(&c2)) = (a_ns.peek(), b_ns.peek()) {
                    if c1 < c2 {
                        a_ns.next();
                    } else if c2 < c1 {
                        b_ns.next();
                    } else {    // c1 == c2
                        if filter(c1) {
                            f(a, b, c1);
                        }
                        a_ns.next();
                        b_ns.next();
                    }
                }
            }
        }
    }

    fn breadth_first(&self, start: u16) -> BreadthFirst<T> {
        BreadthFirst::new(self, start)
    }

    fn edge_triangles(&self, a: u16, b: u16) -> EdgeTriangles<T> {
        EdgeTriangles::new(self, a, b)
    }
}

struct BreadthFirst<'a, T: 'a> {
    g: &'a Graph<T>,
    seen: Box<[bool]>,
    cur_depth: u8,
    cur_verts: Vec<u16>,
    next_verts: Vec<u16>,
}

impl<'a, T> BreadthFirst<'a, T> {
    fn new(g: &'a Graph<T>, start: u16) -> BreadthFirst<'a, T> {
        let mut seen = mk_array(false, g.verts.len());
        seen[start as usize] = true;
        BreadthFirst {
            g: g,
            seen: seen,
            cur_depth: 0,
            cur_verts: vec![start],
            next_verts: vec![],
        }
    }
}

impl<'a, T> Iterator for BreadthFirst<'a, T> {
    type Item = (u16, u8);

    fn next(&mut self) -> Option<(u16, u8)> {
        if self.cur_verts.len() == 0 {
            if self.next_verts.len() == 0 {
                // No verts left in either Vec.
                return None;
            } else {
                // Done with this depth, go to the next one.
                mem::swap(&mut self.cur_verts, &mut self.next_verts);
                self.cur_depth += 1
            }
        }

        let v = self.cur_verts.pop().unwrap();
        for &n in self.g.neighbors(v) {
            if !self.seen[n as usize] {
                self.seen[n as usize] = true;
                self.next_verts.push(n);
            }
        }

        Some((v, self.cur_depth))
    }
}

struct EdgeTriangles<'a, T: 'a> {
    g: &'a Graph<T>,
    a: u16,
    b: u16,
    cs1: Peekable<slice::Iter<'a, u16>>,
    cs2: Peekable<slice::Iter<'a, u16>>,
}

impl<'a, T> EdgeTriangles<'a, T> {
    fn new(g: &'a Graph<T>, a: u16, b: u16) -> EdgeTriangles<'a, T> {
        EdgeTriangles {
            g: g,
            a: a,
            b: b,
            cs1: g.neighbors(a).iter().peekable(),
            cs2: g.neighbors(b).iter().peekable(),
        }
    }
}

impl<'a, T> Iterator for EdgeTriangles<'a, T> {
    type Item = u16;

    fn next(&mut self) -> Option<u16> {
        while let (Some(&&c1), Some(&&c2)) = (self.cs1.peek(), self.cs2.peek()) {
            if c1 < c2 {
                self.cs1.next();
            } else if c2 < c1 {
                self.cs2.next();
            } else {    // c1 == c2
                self.cs1.next();
                self.cs2.next();
                return Some(c1);
            }
        }
        None
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

    fn is_puzzle(&self) -> bool {
        self.area >= AREA_FIRST_PUZZLE
    }
}

pub struct Temporary {
    base: Graph<BaseVert>,
    tunnels: Graph<()>,
    next_area: u32,

    edges: Vec<(V2, V2)>,
    neg_edges: Vec<(V2, V2)>,
    tris: Vec<Triangle>,
    vaults: Vec<Box<Vault>>,
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
        for ns in &mut self.base.edges {
            ns.sort();
        }

        self.tunnels = Graph::new();
        for _ in 0 .. self.base.verts.len() {
            self.tunnels.add_vert(());
        }
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

    fn gen(&mut self, rng: &mut StdRng) {
        struct Path {
            cur: u16,
            level: u8,
        }

        let mut queue = Vec::new();

        let origin = self.base.verts.iter().position(|v| v.pos == ENTRANCE_POS)
                         .expect("ENTRANCE_POS should always be in self.base.verts") as u16;

        for &v in &self.gen_entrance(rng, origin) {
            queue.push(Path { cur: v, level: 0 });
        }

        while queue.len() > 0 {
            let idx = rng.gen_range(0, queue.len());
            let p = queue.swap_remove(idx);

            // Extend the path in some direction(s).
            // 70% chance of 1
            // 25% chance of 2
            //  5% chance of 3
            let r = rng.gen_range(0, 100);
            let count = 1 + (r < 30) as u8 + (r < 5) as u8;
            for _ in 0 .. count {
                // Dig a tunnel in some direction.
                let next = match self.base.choose_neighbor(rng, p.cur,
                                                           |v| self.base.vert(v).area == 0) {
                    Some(x) => x,
                    None => break,
                };
                let dir = self.base.vert(next).pos - self.base.vert(p.cur).pos;

                let len = rng.gen_range(3, 6);
                let tunnel_end = self.gen_tunnel(rng, p.cur, len, dir, p.level as u32);
                info!("dug tunnel {} -> {}", p.cur, tunnel_end);

                // The tunnel hit a dead end.  Put some loot there so the player won't be
                // disappointed.  The tunnel may also be cut short randomly.
                if tunnel_end == p.cur || (p.level > 0 && rng.gen_range(0, 100) < 20) {
                    info!("gen terminal loot at {}", p.cur);
                    self.gen_loot(rng, p.cur, p.level);
                    break;
                }

                // Place something at the end of the tunnel.
                (|| {
                    if rng.gen_range(0, 100) < 40 {
                        if let Some((above, below)) = self.gen_gem_puzzle(rng, p.cur, p.level) {
                            queue.push(Path { cur: above, level: p.level + 2 });
                            // Also try to gen some tunnels that could lead to gems.
                            queue.push(Path { cur: below, level: p.level });
                            queue.push(Path { cur: below, level: p.level });
                            return;
                        }
                    } else {
                        if let Some(next) = self.gen_door(rng, p.cur) {
                            queue.push(Path { cur: next, level: p.level + 1 });
                            return;
                        }
                    }

                    // Couldn't place anything interesting.
                    info!("gen backup loot at {}", tunnel_end);
                    self.gen_loot(rng, tunnel_end, p.level);
                })();

                // Try to place some extra treasure as well.
                continue;
                let opt_v = {
                    let iter = (0 .. self.base.verts.len() as u16)
                                   .filter(|&v| self.base.vert(v).area == AREA_TUNNEL);
                    reservoir_sample(rng, iter)
                };
                if let Some(v) = opt_v {
                    if let Some(n) = self.base.choose_neighbor(rng, p.cur,
                                                               |v| self.base.vert(v).area == 0) {
                        let level = self.base.vert(v).label as u8;
                        let dir = self.base.vert(n).pos - self.base.vert(v).pos;
                        let tunnel_end = self.gen_tunnel(rng, v, 2, dir, level as u32);
                        info!("gen extra loot from {} -> {} -> {}", v, n, tunnel_end);
                        self.gen_loot(rng, tunnel_end, level);
                    }
                }
            }
        }
    }

    fn gen_entrance(&mut self, rng: &mut StdRng, origin: u16) -> [u16; 3] {
        let mut verts = Vec::with_capacity(self.base.neighbors(origin).len() + 1);
        verts.push(origin);
        verts.extend(self.base.neighbors(origin).iter().map(|&x| x)
                         .filter(|&v| !self.base.vert(v).is_puzzle()));
        let verts = verts.into_boxed_slice();

        // Place tunnel edges
        let area = self.mark_area(&verts);
        for &v1 in &*verts {
            for &v2 in self.base.neighbors(v1) {
                if self.base.vert(v2).area == area && v1 < v2 {
                    self.tunnels.add_edge_undir(v1, v2);
                }
            }
        }

        // Place triangles
        {
            let base = &self.base;
            let tris = &mut self.tris;
            base.for_each_triangle_filtered(|v| base.vert(v).area == area, |a, b, c| {
                tris.push(Triangle::new(base.vert(a).pos,
                                        base.vert(b).pos,
                                        base.vert(c).pos));
            });
        }

        // Place vault
        self.vaults.push(Box::new(vault::Entrance::new(ENTRANCE_POS)));

        // Choose outgoing vertexes
        let mut choose = || {
            let v = self.base.choose_neighbor(rng, origin, |v| {
                let v = self.base.vert(v);
                v.area == area && v.label == 0
            }).unwrap();
            self.base.vert_mut(v).label = 1;
            v
        };
        [choose(), choose(), choose()]
    }

    fn step_tunnel(&mut self, rng: &mut StdRng, v: u16, dir: V2) -> Option<u16> {
        let v_pos = self.base.vert(v).pos;
        reservoir_sample_weighted(rng,
                self.base.neighbors(v).iter()
                    .filter(|&&n| self.base.vert(n).area == 0)
                    .map(|&n| {
                        let d = dir.dot(self.base.vert(n).pos - v_pos);
                        (n, if d < 0 { 200 } else { d + 200 })
                    }))
    }

    fn gen_tunnel(&mut self, rng: &mut StdRng, v: u16, len: usize, dir: V2, label: u32) -> u16 {
        let mut cur = v;
        for _ in 0 .. len {
            let next = match self.step_tunnel(rng, cur, dir) {
                Some(x) => x,
                None => break,
            };
            self.tunnels.add_edge_undir(cur, next);
            self.base.vert_mut(next).area = AREA_TUNNEL;
            self.base.vert_mut(next).label = label;
            cur = next;
        }
        cur
    }

    fn gen_door(&mut self, rng: &mut StdRng, start: u16) -> Option<u16> {
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

            self.place_door(rng, path, start, v1, v2, &side_verts);
            return Some(v2);
        }

        // Fell through without finding a place to put the door.
        None
    }

    fn place_door(&mut self,
                  rng: &mut StdRng,
                  path: Vec<u16>,
                  start: u16,
                  entrance: u16,
                  exit: u16,
                  side_verts: &[u16]) {
        let area = self.alloc_area();

        // Add edges for the extra path.
        let mut last = start;
        for &v in &path[1..] {
            self.tunnels.add_edge_undir(last, v);
            self.base.vert_mut(v).area = AREA_TUNNEL;
            last = v;
        }

        // Take ownership of the necessary vertices.
        self.base.vert_mut(entrance).area = area;
        self.base.vert_mut(exit).area = area;
        for &v in side_verts.iter() {
            self.base.vert_mut(v).area = area;
        }

        // Place vault and edges
        let a = entrance;
        let b = exit;
        let a_pos = self.base.vert(a).pos;
        let b_pos = self.base.vert(b).pos;

        let center = (a_pos + b_pos).div_floor(scalar(2));
        self.edges.push((a_pos, center + V2::new(0, 2)));
        self.edges.push((b_pos, center - V2::new(0, 2)));

        self.vaults.push(Box::new(vault::Door::new(center)));

        // Generate triangles and neg_edges
        let ab = b_pos - a_pos;
        // NB: in game coordinates, the Y-axis points down, not up.
        let right = V2::new(-ab.y, ab.x);

        for &c in side_verts {
            let c_conn = self.tunnels.neighbors(c).len() > 0;
            let bc_conn = self.tunnels.has_edge(b, c);
            let ca_conn = self.tunnels.has_edge(c, a);

            let c_pos = self.base.vert(c).pos;

            let on_right = (c_pos - a_pos).dot(right) > 0;
            let ab_mid = center + V2::new(if on_right { 2 } else { -2 }, 0);
            let bc_mid = (b_pos + c_pos).div_floor(scalar(2));
            let ca_mid = (c_pos + a_pos).div_floor(scalar(2));

            // Place triangles as there is a tunnel from entrance to exit.
            self.tris.push(Triangle::new(a_pos, ab_mid, ca_mid));
            self.tris.push(Triangle::new(b_pos, bc_mid, ab_mid));
            self.tris.push(Triangle::new(ab_mid, bc_mid, ca_mid));
            if c_conn {
                self.tris.push(Triangle::new(c_pos, ca_mid, bc_mid));
            }

            // But generate negative edges as if there is no such tunnel.  This prevents the
            // entrance and exit tunnels from being accidentally connected.
            if !ca_conn {
                self.neg_edges.push((ab_mid, ca_mid));
            }
            if !bc_conn {
                self.neg_edges.push((bc_mid, ab_mid));
            }
            if !bc_conn && !ca_conn {
                self.neg_edges.push((ca_mid, bc_mid));
            }
        }
    }

    fn gen_library(&mut self, rng: &mut StdRng, v: u16) -> bool {
        false
    }

    fn gen_loot(&mut self, rng: &mut StdRng, v: u16, level: u8) -> bool {
        info!("placing loot at {} ({:?}), level {}",
              v, self.base.vert(v).pos, level);
        let pos = self.base.vert(v).pos;
        if rng.gen_range(0, 100) < 50 {
            // Chest with keys
            let count = rng.gen_range(1, 3) + rng.gen_range(1, 3);
            let kind = vault::TreasureKind::Chest(count, vault::ChestItem::Key);
            self.vaults.push(Box::new(vault::Treasure::new(pos, kind)));
            return true;
        }

        let hat_weight = if level < 3 { 0 } else { level - 3 + 1 };

        let r = rng.gen_range(0, 100);
        let kind =
            if r < 2 * hat_weight {
                vault::TreasureKind::Chest(1, vault::ChestItem::Hat)
            } else if r < 50 {
                let count = rng.gen_range(6, 13);
                vault::TreasureKind::Chest(count, vault::ChestItem::Book)
            } else if r < 75 {
                vault::TreasureKind::Fountain
            } else {
                vault::TreasureKind::Trophy
            };
        self.vaults.push(Box::new(vault::Treasure::new(pos, kind)));
        true
    }

    fn gen_gem_puzzle(&mut self, rng: &mut StdRng, start: u16, level: u8) -> Option<(u16, u16)> {
        let mut info = None;

        // Big loop to find a place to put everything.
        'top: for (v, depth) in self.base.breadth_first(start) {
            if depth > 3 {
                break;
            }

            if self.base.vert(v).area != 0 {
                continue;
            }
            let v_pos = self.base.vert(v).pos;
            for &n in self.base.neighbors(v) {
                if self.base.vert(n).area != 0 {
                    continue;
                }
                let n_pos = self.base.vert(n).pos;

                // Check that the edge v--n is roughly horizontal.
                let delta = (n_pos - v_pos).abs();
                if delta.x < 8 || delta.x < delta.y * 5 / 2 {
                    continue;
                }

                // Check that the triangles above and below are open.
                let opt_above = self.base.edge_triangles(v, n)
                                    .filter(|&c| self.base.vert(c).area == 0)
                                    .min_by(|&c| self.base.vert(c).pos.y);
                let opt_below = self.base.edge_triangles(v, n)
                                    .filter(|&c| self.base.vert(c).area == 0)
                                    .max_by(|&c| self.base.vert(c).pos.y);
                let (above, below) = match (opt_above, opt_below) {
                    (Some(x), Some(y)) => (x, y),
                    _ => continue,
                };

                // Check that the lower-left and lower-right triangles are also free.
                let opt_corner_v = self.base.edge_triangles(below, v)
                                       .filter(|&c| {
                                           c != n && (c == start || self.base.vert(c).area == 0)
                                       }).nth(0);
                let opt_corner_n = self.base.edge_triangles(below, n)
                                       .filter(|&c| {
                                           c != v && (c == start || self.base.vert(c).area == 0)
                                       }).nth(0);
                if opt_corner_v.is_none() || opt_corner_n.is_none() {
                    continue;
                }

                // Find a path to the bottom of the diamond.
                let path = self.base.a_star(start, below, |v| v.pos, |w,_| {
                    if w == v || w == n || w == above || w == below { 10000 } else { 0 }
                });
                if path.len() > 3 {
                    continue;
                }

                info = Some((v, n, above, below,
                             opt_corner_v.unwrap(), opt_corner_n.unwrap(),
                             path));

                break 'top;
            }
        }

        // Unpack info about the chosen area.
        let (left, right, above, below, left_corner, right_corner, path) = match info {
            Some(x) => x,
            None => return None,
        };

        // Mark vertices.  We do this first so we can capture `base` in a helper closure.
        for &v in &path {
            self.base.vert_mut(v).area = AREA_TUNNEL;
        }
        let area = self.mark_area(&[left, right, above, below, left_corner, right_corner]);

        macro_rules! pos { ($e:expr) => (self.base.vert($e).pos) };

        let (left, right, left_corner, right_corner) =
            if pos!(left).x > pos!(right).x { (right, left, right_corner, left_corner) }
            else { (left, right, left_corner, right_corner) };

        // Add the triangles
        self.tris.push(Triangle::new(pos!(left), pos!(right), pos!(above)));
        self.tris.push(Triangle::new(pos!(left), pos!(right), pos!(below)));
        self.tris.push(Triangle::new(pos!(left), pos!(left_corner), pos!(below)));
        self.tris.push(Triangle::new(pos!(right), pos!(right_corner), pos!(below)));

        // Add the special edges around the triangles.
        let mid_pos = (pos!(left) + pos!(right)).div_floor(scalar(2));
        self.edges.push((mid_pos, pos!(above)));
        self.edges.push((mid_pos, pos!(below)));
        self.neg_edges.push((pos!(left), mid_pos - V2::new(2, 0)));
        self.neg_edges.push((pos!(right), mid_pos + V2::new(2, 0)));
        self.neg_edges.push((pos!(left), (pos!(left) + pos!(left_corner)).div_floor(scalar(2))));
        self.neg_edges.push((pos!(right), (pos!(right) + pos!(right_corner)).div_floor(scalar(2))));

        // Handle the path from `start` to `below`.
        let mut cur = start;
        for &next in &path[1..] {
            self.tunnels.add_edge_undir(cur, next);
            cur = next;
        }

        // Add the actual door and puzzle.
        self.vaults.push(Box::new(vault::Door::new(mid_pos)));

        let gem_center = mid_pos + V2::new(0, 3);
        let mut colors = mk_array(vault::GemSlot::Empty, 3);
        static PRIMARIES: [vault::GemSlot; 3] = [
            vault::GemSlot::Red,
            vault::GemSlot::Yellow,
            vault::GemSlot::Blue,
        ];
        if level == 0 && rng.gen_range(0, 3) < 2 {
            let i = rng.gen_range(0, 3);
            let j = rng.gen_range(1, 3);
            colors[0] = PRIMARIES[i];
            colors[2] = PRIMARIES[(i + j) % 3];
        } else {
            let slot = if rng.gen_range(0, 2) == 0 { 0 } else { 2 };
            let i = rng.gen_range(0, 3);
            colors[slot] = PRIMARIES[i];
        }
        self.vaults.push(Box::new(vault::GemPuzzle::new(gem_center, area, colors)));

        Some((above, below))
    }

    /*
    fn gen_gem_puzzle(&mut self, rng: &mut StdRng, v: u16) -> u16 {
        let verts = simple_blob(&self.base, rng, v, 8);

        // Place tunnel edges
        let area = self.mark_area(&verts);
        for &v1 in &*verts {
            for &v2 in self.base.neighbors(v1) {
                if self.base.vert(v2).area == area && v1 < v2 {
                    self.tunnels.add_edge_undir(v1, v2);
                }
            }
        }

        // Place triangles
        {
            let base = &self.base;
            let tris = &mut self.tris;
            base.for_each_triangle_filtered(|v| base.vert(v).area == area, |a, b, c| {
                tris.push(Triangle::new(base.vert(a).pos,
                                        base.vert(b).pos,
                                        base.vert(c).pos));
            });
        }

        // Choose exit vertex
        let opt_exit = {
            let base = &self.base;
            let iter = verts.iter().map(|&x| x).filter(|&v| {
                let v_pos = base.vert(v).pos;
                let mut above = false;
                let mut space_below = false;
                let mut below_left = false;
                let mut below_right = false;
                for &n in base.neighbors(v) {
                    let d = base.vert(n).pos - v_pos;
                    if base.vert(n).area == area {
                        if d.y >= 10 {
                            space_below = true;
                        }
                        if d.y >= 6 && d.x > 1 {
                            below_right = true;
                        }
                        if d.y >= 6 && d.x < -1 {
                            below_right = true;
                        }
                    }
                    if d.y < 0 && base.vert(n).area == 0 {
                        above = true;
                    }
                }
                above && space_below && below_left && below_right
            });
            reservoir_sample(rng, iter)
        };
        let exit = match opt_exit {
            Some(x) => x,
            None => return v,
        };

        let pos = self.base.vert(exit).pos;
        self.vaults.push(Box::new(vault::Treasure::new(pos, vault::TreasureKind::Trophy)));

        v
    }
    */

    fn make_tris(&mut self) {
        let Temporary { ref base,
                        ref tunnels,
                        ref mut tris,
                        ref mut neg_edges,
                        .. } = *self;
        base.for_each_triangle(|a, b, c| {
            let a_area = base.vert(a).area;
            let b_area = base.vert(b).area;
            let c_area = base.vert(c).area;
            if a_area >= AREA_FIRST_PUZZLE && b_area == a_area && c_area == a_area {
                // Don't do anything for triangles claimed by an area.  The area can fill those in
                // itself.
                return;
            }

            process_triangle(base, tunnels, a, b, c,
                             |a, b, c| tris.push(Triangle::new(a, b, c)),
                             |a, b| neg_edges.push((a, b)));
        });
        info!("generated {} triangles", tris.len());
    }

    fn make_edges(&mut self) {
        for v in 0 .. self.tunnels.verts.len() as u16 {
            for &n in self.tunnels.neighbors(v) {
                // self.tunnels is undirected, so avoid adding duplicates to self.edges.
                if v < n {
                    self.edges.push((self.base.vert(v).pos,
                                     self.base.vert(n).pos));
                }
            }
        }
        info!("generated {} edges", self.edges.len());
    }
}

fn process_triangle<Tri, NegEdge>(base: &Graph<BaseVert>,
                                  tunnels: &Graph<()>,
                                  a: u16, b: u16, c: u16,
                                  mut do_tri: Tri,
                                  mut do_neg_edge: NegEdge)
        where Tri: FnMut(V2, V2, V2),
              NegEdge: FnMut(V2, V2) {
    // Check which vertices and edges are involved in tunnels.
    let a_conn = tunnels.neighbors(a).len() > 0;
    let b_conn = tunnels.neighbors(b).len() > 0;
    let c_conn = tunnels.neighbors(c).len() > 0;

    let ab_conn = tunnels.has_edge(a, b);
    let bc_conn = tunnels.has_edge(b, c);
    let ca_conn = tunnels.has_edge(c, a);

    // Collect positions for vertices and midpoints.
    let a_pos = base.vert(a).pos;
    let b_pos = base.vert(b).pos;
    let c_pos = base.vert(c).pos;

    let ab_mid = (a_pos + b_pos).div_floor(scalar(2));
    let bc_mid = (b_pos + c_pos).div_floor(scalar(2));
    let ca_mid = (c_pos + a_pos).div_floor(scalar(2));

    // Emit triangles
    if a_conn {
        do_tri(a_pos, ab_mid, ca_mid);
    }
    if b_conn {
        do_tri(b_pos, bc_mid, ab_mid);
    }
    if c_conn {
        do_tri(c_pos, ca_mid, bc_mid);
    }
    if ab_conn || bc_conn || ca_conn {
        do_tri(ab_mid, bc_mid, ca_mid);
    }

    // Emit negative edges.
    if (a_conn || bc_conn) && !ab_conn && !ca_conn {
        do_neg_edge(ab_mid, ca_mid);
    }
    if (b_conn || ca_conn) && !bc_conn && !ab_conn {
        do_neg_edge(bc_mid, ab_mid);
    }
    if (c_conn || ab_conn) && !ca_conn && !bc_conn {
        do_neg_edge(ca_mid, bc_mid);
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
            tunnels: Graph::new(),
            next_area: AREA_FIRST_PUZZLE,

            edges: Vec::new(),
            neg_edges: Vec::new(),
            tris: Vec::new(),
            vaults: Vec::new(),
        }
    }

    fn generate(&mut self, tmp: &mut Temporary) {
        tmp.gen_base(&mut self.rng);
        tmp.gen(&mut self.rng);

        tmp.make_edges();
        tmp.make_tris();
    }

    fn save(&mut self, tmp: Temporary, summ: &mut PlaneSummary) {
        summ.edges = tmp.edges;
        summ.neg_edges = tmp.neg_edges;
        summ.tris = tmp.tris;
        summ.vaults = tmp.vaults;
    }
}
