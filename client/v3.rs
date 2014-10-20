use core::prelude::*;
use core::cmp::{min, max};


#[deriving(Eq, PartialEq)]
pub struct V3 {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl V3 {
    pub fn new(x: i32, y: i32, z: i32) -> V3 {
        V3 { x: x, y: y, z: z }
    }

    pub fn map<F: Fn(i32) -> i32>(&self, f: F) -> V3 {
        V3 {
            x: f(self.x),
            y: f(self.y),
            z: f(self.z),
        }
    }

    pub fn zip<F: Fn(i32, i32) -> i32>(&self, other: &V3, f: F) -> V3 {
        V3 {
            x: f(self.x, other.x),
            y: f(self.y, other.y),
            z: f(self.z, other.z),
        }
    }

    pub fn zip3<F: Fn(i32, i32, i32) -> i32>(&self, other1: &V3, other2: &V3, f: F) -> V3 {
        V3 {
            x: f(self.x, other1.x, other2.x),
            y: f(self.y, other1.y, other2.y),
            z: f(self.z, other1.z, other2.z),
        }
    }

    pub fn iter<'a>(&'a self) -> V3Items<'a> {
        V3Items {
            v: self,
            i: 0,
        }
    }

    pub fn abs(&self) -> V3 {
        self.map(|&:a: i32| a.abs())
    }

    pub fn signum(&self) -> V3 {
        self.map(|&:a: i32| a.signum())
    }

    pub fn is_positive(&self) -> V3 {
        self.map(|&:a: i32| (a > 0) as i32)
    }

    pub fn is_negative(&self) -> V3 {
        self.map(|&:a: i32| (a < 0) as i32)
    }

    pub fn is_zero(&self) -> V3 {
        self.map(|&:a: i32| (a == 0) as i32)
    }

    pub fn choose(&self, a: &V3, b: &V3) -> V3 {
        V3 {
            x: if self.x != 0 { a.x } else { b.x },
            y: if self.y != 0 { a.y } else { b.y },
            z: if self.z != 0 { a.z } else { b.z },
        }
    }

    pub fn clamp(&self, low: i32, high: i32) -> V3 {
        self.map(|&: a: i32| max(low, min(high, a)))
    }
}

pub struct V3Items<'a> {
    v: &'a V3,
    i: u8,
}

impl<'a> Iterator<i32> for V3Items<'a> {
    fn next(&mut self) -> Option<i32> {
        self.i += 1;
        if (self.i == 1) {
            Some(self.v.x)
        } else if (self.i == 2) {
            Some(self.v.y)
        } else if (self.i == 3) {
            Some(self.v.z)
        } else {
            None
        }
    }
}

impl FromIterator<i32> for V3 {
    fn from_iter<T: Iterator<i32>>(mut iterator: T) -> V3 {
        let x = iterator.next().unwrap();
        let y = iterator.next().unwrap();
        let z = iterator.next().unwrap();
        V3 { x: x, y: y, z: z }
    }
}

pub fn scalar(w: i32) -> V3 {
    V3 {
        x: w,
        y: w,
        z: w,
    }
}

impl Add<V3, V3> for V3 {
    fn add(&self, other: &V3) -> V3 {
        self.zip(other, |&:a: i32, b: i32| a + b)
    }
}

impl Sub<V3, V3> for V3 {
    fn sub(&self, other: &V3) -> V3 {
        self.zip(other, |&:a: i32, b: i32| a - b)
    }
}

impl Mul<V3, V3> for V3 {
    fn mul(&self, other: &V3) -> V3 {
        self.zip(other, |&:a: i32, b: i32| a * b)
    }
}

impl Div<V3, V3> for V3 {
    fn div(&self, other: &V3) -> V3 {
        self.zip(other, |&:a: i32, b: i32| a / b)
    }
}

impl Rem<V3, V3> for V3 {
    fn rem(&self, other: &V3) -> V3 {
        self.zip(other, |&:a: i32, b: i32| a % b)
    }
}

impl Neg<V3> for V3 {
    fn neg(&self) -> V3 {
        self.map(|&:a: i32| -a)
    }
}

impl Shl<uint, V3> for V3 {
    fn shl(&self, rhs: &uint) -> V3 {
        self.map(|&:a: i32| a << *rhs)
    }
}

impl Shr<uint, V3> for V3 {
    fn shr(&self, rhs: &uint) -> V3 {
        self.map(|&:a: i32| a << *rhs)
    }
}


pub struct RegionPoints {
    cur: V3,
    min: V3,
    max: V3,
}

impl RegionPoints {
    pub fn new(min: V3, max: V3) -> RegionPoints {
        let empty = max.x <= min.x || max.y <= min.y || max.z <= min.z;

        RegionPoints {
            cur: min - V3::new(1, 0, 0),
            min: min,
            max: if !empty { max } else { min },
        }
    }
}

impl Iterator<V3> for RegionPoints {
    fn next(&mut self) -> Option<V3> {
        self.cur.x += 1;
        if self.cur.x >= self.max.x {
            self.cur.x = self.min.x;
            self.cur.y += 1;
            if self.cur.y >= self.max.y {
                self.cur.y = self.min.y;
                self.cur.z += 1;
                if self.cur.z >= self.max.z {
                    return None;
                }
            }
        }
        Some(self.cur)
    }
}
