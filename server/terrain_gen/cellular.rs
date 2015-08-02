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

    pub fn set_fixed(&mut self, pos: V2, val: bool) {
        let bounds = Region::new(scalar(0), self.size);
        self.grid[bounds.index(pos)] = (if val { ACTIVE } else { Cell::empty() }) | FIXED;
    }

    pub fn init<F: FnMut(V2) -> bool>(&mut self, mut f: F) {
        let bounds = Region::new(scalar(0), self.size);
        for pos in bounds.points() {
            let idx = bounds.index(pos);
            if !self.grid[idx].contains(FIXED) {
                self.grid[idx] = if f(pos) { ACTIVE } else { Cell::empty() };
            }
        }
    }

    fn count_neighbors(&self, pos: V2) -> (u8, u8) {
        let bounds = Region::new(scalar(0), self.size);
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
        let bounds = Region::new(scalar(0), self.size);
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
        let bounds = Region::new(scalar(0), self.size);
        self.grid[bounds.index(pos)].contains(ACTIVE)
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
