//! Cellular Automata
//!
//! Provides a general system for running cellular automata.

use std::iter;
use std::mem;

use types::*;

bitflags! {
    flags Cell: u8 {
        const ACTIVE = 0x01,
        const FIXED = 0x02,
    }
}

pub struct CellularGrid {
    grid: Box<[Cell]>,
    tmp: Box<[Cell]>,
    size: V2,
}

impl CellularGrid {
    pub fn new(size: V2) -> CellularGrid {
        let len = (size.x * size.y) as usize;
        let grid = iter::repeat(Cell::empty()).take(len).collect::<Vec<_>>().into_boxed_slice();
        let tmp = iter::repeat(Cell::empty()).take(len).collect::<Vec<_>>().into_boxed_slice();

        CellularGrid {
            grid: grid,
            tmp: tmp,
            size: size,
        }
    }

    pub fn debug(&self) {
        for y in 0 .. self.size.y {
            let mut s = String::new();
            for x in 0 .. self.size.x {
                let idx = self.bounds().index(V2::new(x, y));
                s.push_str(&format!("{:x}", self.grid[idx].bits()));
            }
            info!("{}", s);
        }
    }

    pub fn bounds(&self) -> Region<V2> {
        Region::new(scalar(0), self.size)
    }

    pub fn set_fixed(&mut self, pos: V2, val: bool) {
        let cell_val = (if val { ACTIVE } else { Cell::empty() }) | FIXED;
        self.grid[self.bounds().index(pos)] = cell_val;
    }

    pub fn set(&mut self, pos: V2, val: bool) {
        let cell_val = if val { ACTIVE } else { Cell::empty() };
        self.grid[self.bounds().index(pos)] = cell_val;
    }

    pub fn init<F: FnMut(V2) -> bool>(&mut self, mut f: F) {
        let bounds = self.bounds();
        for pos in bounds.points() {
            let idx = bounds.index(pos);
            if !self.grid[idx].contains(FIXED) {
                self.grid[idx] = if f(pos) { ACTIVE } else { Cell::empty() };
            }
        }
    }

    fn count_neighbors(&self, pos: V2) -> (u8, u8) {
        let bounds = self.bounds();
        let mut active = 0;
        let mut total = 0;
        for &d in &DIRS {
            if bounds.contains(pos + d) {
                active += self.grid[bounds.index(pos + d)].contains(ACTIVE) as u8;
                total += 1;
            }
        }
        (active, total)
    }

    pub fn step<F>(&mut self, mut f: F)
            where F: FnMut(bool, u8, u8) -> bool {
        let bounds = self.bounds();
        for pos in bounds.points() {
            let idx = bounds.index(pos);
            if self.grid[idx].contains(FIXED) {
                self.tmp[idx] = self.grid[idx];
            } else {
                let (active, total) = self.count_neighbors(pos);
                if f(self.grid[idx].contains(ACTIVE), active, total) {
                    self.tmp[idx] = ACTIVE;
                } else {
                    self.tmp[idx] = Cell::empty();
                }
            }
        }

        mem::swap(&mut self.grid, &mut self.tmp);
    }

    pub fn get(&self, pos: V2) -> bool {
        self.grid[self.bounds().index(pos)].contains(ACTIVE)
    }
}

static DIRS: [V2; 8] = [
    V2 { x:  1, y:  0 },
    V2 { x:  1, y:  1 },
    V2 { x:  0, y:  1 },
    V2 { x: -1, y:  1 },
    V2 { x: -1, y:  0 },
    V2 { x: -1, y: -1 },
    V2 { x:  0, y: -1 },
    V2 { x:  1, y: -1 },
];
