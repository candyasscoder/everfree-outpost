use core::prelude::*;
use core::cmp::{min, max};
use core::fmt;
use core::iter::FromIterator;
use core::num::SignedInt;
use core::ops::{Add, Sub, Mul, Div, Rem, Neg, Shl, Shr, BitAnd, BitOr, BitXor, Not};


#[derive(Copy, Eq, PartialEq, Show)]
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

#[derive(Copy, Eq, PartialEq, Clone)]
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

    pub fn with_x(self, val: i32) -> V3 {
        self.with(Axis::X, val)
    }

    pub fn with_y(self, val: i32) -> V3 {
        self.with(Axis::Y, val)
    }

    pub fn with_z(self, val: i32) -> V3 {
        self.with(Axis::Z, val)
    }
}

impl Vn for V3 {
    type Axis = Axis;

    fn from_fn<F: FnMut(Axis) -> i32>(mut f: F) -> V3 {
        let x = f(Axis::X);
        let y = f(Axis::Y);
        let z = f(Axis::Z);
        V3::new(x, y, z)
    }

    fn get(self, axis: Axis) -> i32 {
        match axis {
            Axis::X => self.x,
            Axis::Y => self.y,
            Axis::Z => self.z,
        }
    }

    fn for_axes<F: FnMut(Axis)>(mut f: F) {
        f(Axis::X);
        f(Axis::Y);
        f(Axis::Z);
    }
}

pub trait Vn: Sized+Copy {
    type Axis: Eq+Copy;

    fn from_fn<F: FnMut(<Self as Vn>::Axis) -> i32>(f: F) -> Self;
    fn get(self, axis: <Self as Vn>::Axis) -> i32;
    fn for_axes<F: FnMut(<Self as Vn>::Axis)>(f: F);

    fn on_axis(axis: <Self as Vn>::Axis, mag: i32) -> Self {
        <Self as Vn>::from_fn(|a| if a == axis { mag } else { 0 })
    }

    fn on_dir_axis(dir_axis: (<Self as Vn>::Axis, bool), mag: i32) -> Self {
        let (axis, neg) = dir_axis;
        <Self as Vn>::on_axis(axis, if neg { -mag } else { mag })
    }

    fn map<F: FnMut(i32) -> i32>(self, mut f: F) -> Self {
        <Self as Vn>::from_fn(|a| f(self.get(a)))
    }

    fn zip<F: FnMut(i32, i32) -> i32>(self, other: Self, mut f: F) -> Self {
        <Self as Vn>::from_fn(|a| f(self.get(a), other.get(a)))
    }

    fn zip3<F: FnMut(i32, i32, i32) -> i32>(self, other1: Self, other2: Self, mut f: F) -> Self {
        <Self as Vn>::from_fn(|a| f(self.get(a), other1.get(a), other2.get(a)))
    }

    fn dot(self, other: Self) -> i32 {
        let mut sum = 0;
        <Self as Vn>::for_axes(|a| sum += self.get(a) * other.get(a));
        sum
    }

    fn get_dir(self, dir_axis: (<Self as Vn>::Axis, bool)) -> i32 {
        let (axis, neg) = dir_axis;
        if neg { -self.get(axis) } else { self.get(axis) }
    }

    fn get_if_pos(self, dir_axis: (<Self as Vn>::Axis, bool)) -> i32 {
        let (axis, neg) = dir_axis;
        if neg { 0 } else { self.get(axis) }
    }

    fn only(self, axis: <Self as Vn>::Axis) -> Self {
        <Self as Vn>::on_axis(axis, self.get(axis))
    }

    fn abs(self) -> Self {
        self.map(|x| x.abs())
    }

    fn signum(self) -> Self {
        self.map(|x| x.signum())
    }

    fn is_positive(self) -> Self {
        self.map(|x| (x > 0) as i32)
    }

    fn is_negative(self) -> Self {
        self.map(|x| (x < 0) as i32)
    }

    fn is_zero(self) -> Self {
        self.map(|x| (x == 0) as i32)
    }

    fn choose(self, a: Self, b: Self) -> Self {
        self.zip3(a, b, |x, a, b| if x != 0 { a } else { b })
    }

    fn clamp(self, low: i32, high: i32) -> Self {
        self.map(|x| max(low, min(high, x)))
    }

    fn with(self, axis: <Self as Vn>::Axis, mag: i32) -> Self {
        <Self as Vn>::from_fn(|a| if a == axis { mag } else { self.get(a) })
    }

    fn div_floor(self, other: Self) -> Self {
        self.zip(other, |a, b| div_floor(a, b))
    }


    fn add(self, other: Self) -> Self {
        self.zip(other, |a, b| a + b)
    }

    fn sub(self, other: Self) -> Self {
        self.zip(other, |a, b| a - b)
    }

    fn mul(self, other: Self) -> Self {
        self.zip(other, |a, b| a * b)
    }

    fn div(self, other: Self) -> Self {
        self.zip(other, |a, b| a / b)
    }

    fn rem(self, other: Self) -> Self {
        self.zip(other, |a, b| a % b)
    }

    fn neg(self) -> Self {
        self.map(|x| -x)
    }

    fn shl(self, amount: usize) -> Self {
        self.map(|x| x << amount)
    }

    fn shr(self, amount: usize) -> Self {
        self.map(|x| x >> amount)
    }

    fn bitand(self, other: Self) -> Self {
        self.zip(other, |a, b| a & b)
    }

    fn bitor(self, other: Self) -> Self {
        self.zip(other, |a, b| a | b)
    }

    fn bitxor(self, other: Self) -> Self {
        self.zip(other, |a, b| a ^ b)
    }

    fn not(self) -> Self {
        self.map(|x| !x)
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

impl<'a> Iterator for V3Items<'a> {
    type Item = i32;
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
    fn from_iter<I: Iterator<Item=i32>>(mut iterator: I) -> V3 {
        let x = iterator.next().unwrap();
        let y = iterator.next().unwrap();
        let z = iterator.next().unwrap();
        V3 { x: x, y: y, z: z }
    }
}

pub fn scalar<V: Vn>(w: i32) -> V {
    <V as Vn>::from_fn(|_| w)
}


macro_rules! impl_Vn_binop {
    ($vec:ty, $op:ident, $method:ident) => {
        impl $op<$vec> for $vec {
            type Output = $vec;
            fn $method(self, other: $vec) -> $vec {
                <$vec as Vn>::$method(self, other)
            }
        }
    };
}

macro_rules! impl_Vn_unop {
    ($vec:ty, $op:ident, $method:ident) => {
        impl $op for $vec {
            type Output = $vec;
            fn $method(self) -> $vec {
                <$vec as Vn>::$method(self)
            }
        }
    };
}

macro_rules! impl_Vn_shift_op {
    ($vec:ty, $op:ident, $method:ident) => {
        impl $op<usize> for $vec {
            type Output = $vec;
            fn $method(self, amount: usize) -> $vec {
                <$vec as Vn>::$method(self, amount)
            }
        }
    };
}

macro_rules! impl_Vn_ops {
    ($vec:ty) => {
        impl_Vn_binop!($vec, Add, add);
        impl_Vn_binop!($vec, Sub, sub);
        impl_Vn_binop!($vec, Mul, mul);
        impl_Vn_binop!($vec, Div, div);
        impl_Vn_binop!($vec, Rem, rem);
        impl_Vn_unop!($vec, Neg, neg);

        impl_Vn_shift_op!($vec, Shl, shl);
        impl_Vn_shift_op!($vec, Shr, shr);

        impl_Vn_binop!($vec, BitAnd, bitand);
        impl_Vn_binop!($vec, BitOr, bitor);
        impl_Vn_binop!($vec, BitXor, bitxor);
        impl_Vn_unop!($vec, Not, not);
    };
}


impl_Vn_ops!(V3);


#[derive(Copy, Eq, PartialEq, Clone)]
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
    pub fn contains(&self, point: V3) -> bool {
        point.x >= self.min.x && point.x < self.max.x &&
        point.y >= self.min.y && point.y < self.max.y &&
        point.z >= self.min.z && point.z < self.max.z
    }

    #[inline]
    pub fn contains_inclusive(&self, point: V3) -> bool {
        point.x >= self.min.x && point.x <= self.max.x &&
        point.y >= self.min.y && point.y <= self.max.y &&
        point.z >= self.min.z && point.z <= self.max.z
    }

    #[inline]
    pub fn join(&self, other: Region) -> Region {
        Region::new(self.min.zip(other.min, |&:a:i32, b: i32| min(a, b)),
                    self.max.zip(other.max, |&:a:i32, b: i32| max(a, b)))
    }

    #[inline]
    pub fn intersect(&self, other: Region) -> Region {
        Region::new(self.min.zip(other.min, |&:a:i32, b: i32| max(a, b)),
                    self.max.zip(other.max, |&:a:i32, b: i32| min(a, b)))
    }

    #[inline]
    pub fn overlaps(&self, other: Region) -> bool {
        let size = self.intersect(other).size();
        size.x > 0 && size.y > 0 && size.z > 0
    }

    #[inline]
    pub fn div_round(&self, rhs: i32) -> Region {
        Region::new(self.min / scalar(rhs),
                    (self.max + scalar(rhs - 1)) / scalar(rhs))
    }

    #[inline]
    pub fn div_round_signed(&self, rhs: i32) -> Region {
        Region::new(self.min.div_floor(scalar(rhs)),
                    (self.max + scalar(rhs - 1)).div_floor(scalar(rhs)))
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
    pub fn expand(&self, amount: V3) -> Region {
        Region::new(self.min - amount, self.max + amount)
    }

    #[inline]
    pub fn clamp_point(&self, point: V3) -> V3 {
        let x = max(self.min.x, min(self.max.x, point.x));
        let y = max(self.min.y, min(self.max.y, point.y));
        let z = max(self.min.z, min(self.max.z, point.z));
        V3::new(x, y, z)
    }

    #[inline]
    pub fn index(&self, point: V3) -> usize {
        let dx = (self.max.x - self.min.x) as usize;
        let dy = (self.max.y - self.min.y) as usize;
        let offset = point - self.min;
        let x = offset.x as usize;
        let y = offset.y as usize;
        let z = offset.z as usize;
        (z * dy + y) * dx + x
    }
}

impl Add<V3> for Region {
    type Output = Region;
    fn add(self, other: V3) -> Region {
        Region::new(self.min + other, self.max + other)
    }
}

impl Sub<V3> for Region {
    type Output = Region;
    fn sub(self, other: V3) -> Region {
        Region::new(self.min - other, self.max - other)
    }
}

impl Mul<V3> for Region {
    type Output = Region;
    fn mul(self, other: V3) -> Region {
        Region::new(self.min * other, self.max * other)
    }
}

impl Div<V3> for Region {
    type Output = Region;
    fn div(self, other: V3) -> Region {
        Region::new(self.min / other, self.max / other)
    }
}

impl Rem<V3> for Region {
    type Output = Region;
    fn rem(self, other: V3) -> Region {
        Region::new(self.min % other, self.max % other)
    }
}

impl Neg for Region {
    type Output = Region;
    fn neg(self) -> Region {
        Region::new(-self.min, -self.max)
    }
}

impl Shl<usize> for Region {
    type Output = Region;
    fn shl(self, rhs: usize) -> Region {
        Region::new(self.min << rhs, self.max << rhs)
    }
}

impl Shr<usize> for Region {
    type Output = Region;
    fn shr(self, rhs: usize) -> Region {
        Region::new(self.min >> rhs, self.max >> rhs)
    }
}


#[derive(Copy)]
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

impl Iterator for RegionPoints {
    type Item = V3;
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
