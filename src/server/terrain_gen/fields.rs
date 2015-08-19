use rand::Rng;

use types::*;

use terrain_gen::Field;

use super::PointRng;


/// A field that has the same constant value at every point.
pub struct ConstantField(pub i32);

impl Field for ConstantField {
    fn get_value(&self, _: V2) -> i32 {
        self.0
    }

    fn get_region(&self, _: Region2, buf: &mut [i32]) {
        for x in buf.iter_mut() {
            *x = self.0
        }
    }
}


/// A field that has a random value (within a given range) at every point.
pub struct RandomField {
    seed: u64,
    min: i32,
    max: i32,
}

impl RandomField {
    pub fn new(seed: u64, min: i32, max: i32) -> RandomField {
        RandomField {
            seed: seed,
            min: min,
            max: max,
        }
    }
}

impl Field for RandomField {
    fn get_value(&self, pos: V2) -> i32 {
        PointRng::new(self.seed, pos, 0).gen_range(self.min, self.max)
    }
}


/// A field that has either 1 or 0, depending on whether the value of an underlying field is within
/// a given range.
pub struct FilterField<F: Field> {
    base: F,
    min: i32,
    max: i32,
}

impl<F: Field> FilterField<F> {
    pub fn new(base: F, min: i32, max: i32) -> FilterField<F> {
        FilterField {
            base: base,
            min: min,
            max: max,
        }
    }
}

impl<F: Field> Field for FilterField<F> {
    fn get_value(&self, pos: V2) -> i32 {
        let val = self.base.get_value(pos);
        if self.min <= val && val < self.max {
            1
        } else {
            0
        }
    }

    fn get_region(&self, bounds: Region2, buf: &mut [i32]) {
        self.base.get_region(bounds, buf);
        for x in buf.iter_mut() {
            if self.min <= *x && *x < self.max {
                *x = 1;
            } else {
                *x = 0;
            }
        }
    }
}


/// A field that has a tile shape number at every point, based on whether the surrounding points
/// have zero or nonzero value in some underlying field.
pub struct BorderField<F: Field> {
    base: F,
}

impl<F: Field> BorderField<F> {
    pub fn new(base: F) -> BorderField<F> {
        BorderField {
            base: base,
        }
    }
}

impl<F: Field> Field for BorderField<F> {
    fn get_value(&self, pos: V2) -> i32 {
        if self.base.get_value(pos) == 0 {
            // Center point is not inside the shape, so return 'outside'.
            return 0;
        }

        let bits = collect_bits(pos, |p| self.base.get_value(p));
        BORDER_SHAPE_TABLE[bits as usize] as i32
    }

    fn get_region(&self, bounds: Region2, buf: &mut [i32]) {
        use std::iter::repeat;

        let border_bounds = bounds.expand(scalar(1));
        let mut border_buf: Vec<_> = repeat(0).take(border_bounds.volume() as usize).collect();
        self.base.get_region(border_bounds, &mut *border_buf);

        for p in bounds.points() {
            if border_buf[border_bounds.index(p)] != 0 {
                let bits = collect_bits(p, |p| border_buf[border_bounds.index(p)]);
                buf[bounds.index(p)] = BORDER_SHAPE_TABLE[bits as usize] as i32;
            } else {
                buf[bounds.index(p)] = 0;
            }
        }
    }
}

fn collect_bits<F: Fn(V2) -> i32>(center: V2, get: F) -> u8 {
    let mut result = 0;
    for (i, &offset) in OFFSET_TABLE.iter().enumerate() {
        if get(center + offset) != 0 {
            result |= 1 << i
        }
    }
    result
}

// Generated 2015-03-20 07:49:07 by util/gen_border_shape_table.py
const BORDER_SHAPE_TABLE: [u8; 256] = [
     0,  0,  0,  0,  0,  0,  0, 10,  0,  0,  0,  0,  0,  0,  0, 10,
     0,  0,  0,  0,  0,  0,  0, 10,  0,  0,  0,  0, 11, 11, 11,  2,
     0,  0,  0,  0,  0,  0,  0, 10,  0,  0,  0,  0,  0,  0,  0, 10,
     0,  0,  0,  0,  0,  0,  0, 10,  0,  0,  0,  0, 11, 11, 11,  2,
     0,  0,  0,  0,  0,  0,  0, 10,  0,  0,  0,  0,  0,  0,  0, 10,
     0,  0,  0,  0,  0,  0,  0, 10,  0,  0,  0,  0, 11, 11, 11,  2,
     0,  0,  0,  0,  0,  0,  0, 10,  0,  0,  0,  0,  0,  0,  0, 10,
    13, 13, 13, 13, 13, 13, 13, 14, 13, 13, 13, 13,  4,  4,  4,  7,
     0,  0,  0,  0,  0,  0,  0, 10,  0,  0,  0,  0,  0,  0,  0, 10,
     0,  0,  0,  0,  0,  0,  0, 10,  0,  0,  0,  0, 11, 11, 11,  2,
     0,  0,  0,  0,  0,  0,  0, 10,  0,  0,  0,  0,  0,  0,  0, 10,
     0,  0,  0,  0,  0,  0,  0, 10,  0,  0,  0,  0, 11, 11, 11,  2,
     0, 12,  0, 12,  0, 12,  0,  5,  0, 12,  0, 12,  0, 12,  0,  5,
     0, 12,  0, 12,  0, 12,  0,  5,  0, 12,  0, 12, 11, 15, 11,  6,
     0, 12,  0, 12,  0, 12,  0,  5,  0, 12,  0, 12,  0, 12,  0,  5,
    13,  3, 13,  3, 13,  3, 13,  8, 13,  3, 13,  3,  4,  9,  4,  1,
];

// Generated 2015-03-20 06:44:51 by util/gen_border_shape_table.py
const OFFSET_TABLE: [V2; 8] = [
    V2 { x:  1, y:  0 },
    V2 { x:  1, y:  1 },
    V2 { x:  0, y:  1 },
    V2 { x: -1, y:  1 },
    V2 { x: -1, y:  0 },
    V2 { x: -1, y: -1 },
    V2 { x:  0, y: -1 },
    V2 { x:  1, y: -1 },
];


