use std::iter;
use std::num::{Zero, One};
use std::ops::{Add, Sub, BitAnd, BitOr, BitXor, Not, Shl, Shr};

use libserver_types::*;


pub struct PatternGrid<T> {
    arr: Box<[T]>,
    size: V2,
    cell_bits: u8,
    row_bits: u8,
}

pub trait BitNum: Copy + Eq + Zero + One +
                  Add<Output=Self> + Sub<Output=Self> +
                  BitAnd<Output=Self> + BitOr<Output=Self> + BitXor<Output=Self> +
                  Not<Output=Self> + Shl<u8, Output=Self> + Shr<u8, Output=Self> {}

impl<T: Copy + Eq + Zero + One +
        Add<Output=T> + Sub<Output=T> +
        BitAnd<Output=T> + BitOr<Output=T> + BitXor<Output=T> +
        Not<Output=T> + Shl<u8, Output=T> + Shr<u8, Output=T>> BitNum for T {}

impl<T: BitNum> PatternGrid<T> {
    pub fn new(size: V2, cell_bits: u8, pattern_size: V2) -> PatternGrid<T> {
        let len = (size.x * size.y) as usize;
        let arr = iter::repeat(Zero::zero()).take(len).collect::<Vec<T>>().into_boxed_slice();
        PatternGrid {
            arr: arr,
            size: size,
            cell_bits: cell_bits,
            row_bits: cell_bits * pattern_size.x as u8,
        }
    }

    pub fn bounds(&self) -> Region<V2> {
        Region::new(scalar(0), self.size)
    }

    pub fn init<F>(&mut self, mut f: F)
            where F: FnMut(V2) -> T {
        let bounds = self.bounds();

        let len = (self.size.x * self.size.y) as usize;
        let mut tmp = iter::repeat(Zero::zero()).take(len).collect::<Vec<T>>().into_boxed_slice();
        for y in 0 .. self.size.y {
            let mut acc = Zero::zero();
            for x in 0 .. self.size.x {
                let pos = V2::new(x, y);
                acc = (acc << self.cell_bits) | f(pos);
                tmp[bounds.index(pos)] = acc;
            }
        }

        let row_mask = (<T as One>::one() << self.row_bits) - One::one();
        for x in 0 .. self.size.x {
            let mut acc: T = Zero::zero();
            for y in 0 .. self.size.y {
                let pos = V2::new(x, y);
                acc = (acc << self.row_bits) | (tmp[bounds.index(pos)] & row_mask);
                self.arr[bounds.index(pos)] = acc;
            }
        }
    }

    pub fn find(&self, value: T, mask: T) -> Vec<V2> {
        let mut v = Vec::new();

        let bounds = self.bounds();
        for pos in bounds.points() {
            let acc = self.arr[bounds.index(pos)];
            if acc & mask == value {
                v.push(pos);
            }
        }

        v
    }
}
