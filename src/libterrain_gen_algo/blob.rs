//! Blob Generation
//!
//! Expands outwards at random from a set of initial points until a specified amount of space is
//! covered.

use std::collections::HashSet;
use std::iter;
use rand::Rng;

use libserver_types::*;


pub struct BlobGrid {
    grid: Box<[bool]>,
    area_covered: usize,
    pending: Vec<V2>,
    pending_set: HashSet<V2>,
    size: V2,
}

impl BlobGrid {
    pub fn new(size: V2) -> BlobGrid {
        let len = (size.x * size.y) as usize;
        let grid = iter::repeat(false).take(len).collect::<Vec<_>>().into_boxed_slice();

        BlobGrid {
            grid: grid,
            area_covered: 0,
            pending: Vec::new(),
            pending_set: HashSet::new(),
            size: size,
        }
    }

    pub fn debug(&self) {
        for y in 0 .. self.size.y {
            let mut s = String::new();
            for x in 0 .. self.size.x {
                let idx = self.bounds().index(V2::new(x, y));
                s.push_str(if self.grid[idx] { "#" } else { "." });
            }
            info!("{}", s);
        }
    }

    pub fn bounds(&self) -> Region<V2> {
        Region::new(scalar(0), self.size)
    }

    fn maybe_enqueue(&mut self, pos: V2) {
        if !self.bounds().contains(pos) ||
           self.grid[self.bounds().index(pos)] == true ||
           self.pending_set.contains(&pos) {
            return;
        }

        self.pending.push(pos);
        self.pending_set.insert(pos);
    }

    pub fn add_point(&mut self, pos: V2) {
        if !self.bounds().contains(pos) {
            return;
        }

        self.grid[self.bounds().index(pos)] = true;
        self.area_covered += 1;
        for &dir in &DIRS {
            self.maybe_enqueue(pos + dir);
        }
    }

    pub fn expand<R: Rng>(&mut self, rng: &mut R, area: usize) {
        while self.area_covered < area {
            let idx = rng.gen_range(0, self.pending.len());
            let pos = self.pending.swap_remove(idx);
            self.pending_set.remove(&pos);
            self.add_point(pos);
        }
    }

    pub fn get(&self, pos: V2) -> bool {
        if !self.bounds().contains(pos) {
            return false;
        }

        self.grid[self.bounds().index(pos)]
    }
}

const DIRS: [V2; 4] = [
    V2 { x:  1, y:  0 },
    V2 { x:  0, y:  1 },
    V2 { x: -1, y:  0 },
    V2 { x:  0, y: -1 },
];
