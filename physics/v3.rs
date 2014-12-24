use core::prelude::*;
use core::cmp::{min, max};
use core::fmt;
use core::num::SignedInt;


#[deriving(Eq, PartialEq, Show)]
pub enum Axis {
    X,
    Y,
    Z,
}

pub type DirAxis = (Axis, bool);
pub mod DirAxis {
    #![allow(non_snake_case, non_upper_case_globals)]
    use super::{Axis, DirAxis};
    pub const PosX: DirAxis = (Axis::X, false);
    pub const PosY: DirAxis = (Axis::Y, false);
    pub const PosZ: DirAxis = (Axis::Z, false);
    pub const NegX: DirAxis = (Axis::X, true);
    pub const NegY: DirAxis = (Axis::Y, true);
    pub const NegZ: DirAxis = (Axis::Z, true);
}

#[deriving(Eq, PartialEq, Clone)]
pub struct V3 {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

impl fmt::Show for V3 {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        (self.x, self.y, self.z).fmt(f)
    }
}

impl V3 {
    pub fn new(x: i32, y: i32, z: i32) -> V3 {
        V3 { x: x, y: y, z: z }
    }

    pub fn on_axis(axis: Axis, mag: i32) -> V3 {
        match axis {
            Axis::X => V3::new(mag, 0, 0),
            Axis::Y => V3::new(0, mag, 0),
            Axis::Z => V3::new(0, 0, mag),
        }
    }

    pub fn on_dir_axis(dir_axis: DirAxis, mag: i32) -> V3 {
        let (axis, neg) = dir_axis;
        let result = V3::on_axis(axis, mag);
        if !neg { result } else { -result }
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

    pub fn dot(&self, other: &V3) -> i32 {
        self.x * other.x +
        self.y * other.y +
        self.z * other.z
    }

    pub fn get(&self, axis: Axis) -> i32 {
        match axis {
            Axis::X => self.x,
            Axis::Y => self.y,
            Axis::Z => self.z,
        }
    }

    pub fn get_dir(&self, dir_axis: DirAxis) -> i32 {
        let (axis, neg) = dir_axis;
        if !neg { self.get(axis) } else { -self.get(axis) }
    }

    pub fn get_if_pos(&self, dir_axis: DirAxis) -> i32 {
        let (axis, neg) = dir_axis;
        if !neg { self.get(axis) } else { 0 }
    }

    pub fn only(&self, axis: Axis) -> V3 {
        V3::on_axis(axis, self.get(axis))
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

    pub fn with_x(&self, new_x: i32) -> V3 {
        V3::new(new_x, self.y, self.z)
    }

    pub fn with_y(&self, new_y: i32) -> V3 {
        V3::new(self.x, new_y, self.z)
    }

    pub fn with_z(&self, new_z: i32) -> V3 {
        V3::new(self.x, self.y, new_z)
    }

    pub fn with(&self, axis: Axis, new: i32) -> V3 {
        match axis {
            Axis::X => self.with_x(new),
            Axis::Y => self.with_y(new),
            Axis::Z => self.with_z(new),
        }
    }

    pub fn div_floor(&self, other: &V3) -> V3 {
        self.zip(other, |&: a: i32, b: i32| {
            div_floor(a, b)
        })
    }
}

fn div_floor(a: i32, b: i32) -> i32 {
    if b < 0 {
        return div_floor(-a, -b);
    }

    if a < 0 {
        (a - (b - 1)) / b
    } else {
        a / b
    }
}

pub struct V3Items<'a> {
    v: &'a V3,
    i: u8,
}

impl<'a> Iterator<i32> for V3Items<'a> {
    fn next(&mut self) -> Option<i32> {
        self.i += 1;
        if self.i == 1 {
            Some(self.v.x)
        } else if self.i == 2 {
            Some(self.v.y)
        } else if self.i == 3 {
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
    #[inline]
    fn div(&self, other: &V3) -> V3 {
        self.zip(other, |&:a: i32, b: i32| a / b)
    }
}

impl Rem<V3, V3> for V3 {
    #[inline]
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
        self.map(|&:a: i32| a >> *rhs)
    }
}

impl BitAnd<V3, V3> for V3 {
    fn bitand(&self, other: &V3) -> V3 {
        self.zip(other, |&:a: i32, b: i32| a & b)
    }
}

impl BitOr<V3, V3> for V3 {
    fn bitor(&self, other: &V3) -> V3 {
        self.zip(other, |&:a: i32, b: i32| a | b)
    }
}

impl BitXor<V3, V3> for V3 {
    fn bitxor(&self, other: &V3) -> V3 {
        self.zip(other, |&:a: i32, b: i32| a ^ b)
    }
}

impl Not<V3> for V3 {
    fn not(&self) -> V3 {
        self.map(|&:a: i32| !a)
    }
}


#[deriving(Eq, PartialEq, Clone)]
pub struct Region {
    pub min: V3,
    pub max: V3,
}

impl fmt::Show for Region {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        (self.min, self.max).fmt(f)
    }
}

impl Region {
    #[inline]
    pub fn new(min: V3, max: V3) -> Region {
        Region { min: min, max: max }
    }

    #[inline]
    pub fn around(center: V3, radius: i32) -> Region {
        Region::new(center - scalar(radius),
                    center + scalar(radius))
    }

    #[inline]
    pub fn points(&self) -> RegionPoints {
        RegionPoints::new(self.min, self.max)
    }

    #[inline]
    pub fn size(&self) -> V3 {
        self.max - self.min
    }

    #[inline]
    pub fn volume(&self) -> i32 {
        let size = self.size();
        size.x * size.y * size.z
    }

    #[inline]
    pub fn contains(&self, point: &V3) -> bool {
        point.x >= self.min.x && point.x < self.max.x &&
        point.y >= self.min.y && point.y < self.max.y &&
        point.z >= self.min.z && point.z < self.max.z
    }

    #[inline]
    pub fn contains_inclusive(&self, point: &V3) -> bool {
        point.x >= self.min.x && point.x <= self.max.x &&
        point.y >= self.min.y && point.y <= self.max.y &&
        point.z >= self.min.z && point.z <= self.max.z
    }

    #[inline]
    pub fn join(&self, other: &Region) -> Region {
        Region::new(self.min.zip(&other.min, |&:a:i32, b: i32| min(a, b)),
                    self.max.zip(&other.max, |&:a:i32, b: i32| max(a, b)))
    }

    #[inline]
    pub fn intersect(&self, other: &Region) -> Region {
        Region::new(self.min.zip(&other.min, |&:a:i32, b: i32| max(a, b)),
                    self.max.zip(&other.max, |&:a:i32, b: i32| min(a, b)))
    }

    #[inline]
    pub fn div_round(&self, rhs: i32) -> Region {
        Region::new(self.min / scalar(rhs),
                    (self.max + scalar(rhs - 1)) / scalar(rhs))
    }

    #[inline]
    pub fn div_round_signed(&self, rhs: i32) -> Region {
        Region::new(self.min.div_floor(&scalar(rhs)),
                    (self.max + scalar(rhs - 1)).div_floor(&scalar(rhs)))
    }

    #[inline]
    pub fn with_zs(&self, min_z: i32, max_z: i32) -> Region {
        Region::new(self.min.with_z(min_z), self.max.with_z(max_z))
    }

    #[inline]
    pub fn flatten(&self, depth: i32) -> Region {
        self.with_zs(self.min.z, self.min.z + depth)
    }

    #[inline]
    pub fn expand(&self, amount: &V3) -> Region {
        Region::new(self.min - *amount, self.max + *amount)
    }

    #[inline]
    pub fn clamp_point(&self, point: &V3) -> V3 {
        let x = max(self.min.x, min(self.max.x, point.x));
        let y = max(self.min.y, min(self.max.y, point.y));
        let z = max(self.min.z, min(self.max.z, point.z));
        V3::new(x, y, z)
    }

    #[inline]
    pub fn index(&self, point: &V3) -> uint {
        let dx = (self.max.x - self.min.x) as uint;
        let dy = (self.max.y - self.min.y) as uint;
        let offset = *point - self.min;
        let x = offset.x as uint;
        let y = offset.y as uint;
        let z = offset.z as uint;
        (z * dy + y) * dx + x
    }
}

impl Add<V3, Region> for Region {
    fn add(&self, other: &V3) -> Region {
        Region::new(self.min + *other, self.max + *other)
    }
}

impl Sub<V3, Region> for Region {
    fn sub(&self, other: &V3) -> Region {
        Region::new(self.min - *other, self.max - *other)
    }
}

impl Mul<V3, Region> for Region {
    fn mul(&self, other: &V3) -> Region {
        Region::new(self.min * *other, self.max * *other)
    }
}

impl Div<V3, Region> for Region {
    fn div(&self, other: &V3) -> Region {
        Region::new(self.min / *other, self.max / *other)
    }
}

impl Rem<V3, Region> for Region {
    fn rem(&self, other: &V3) -> Region {
        Region::new(self.min % *other, self.max % *other)
    }
}

impl Neg<Region> for Region {
    fn neg(&self) -> Region {
        Region::new(-self.min, -self.max)
    }
}

impl Shl<uint, Region> for Region {
    fn shl(&self, rhs: &uint) -> Region {
        Region::new(self.min << *rhs, self.max << *rhs)
    }
}

impl Shr<uint, Region> for Region {
    fn shr(&self, rhs: &uint) -> Region {
        Region::new(self.min >> *rhs, self.max >> *rhs)
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
