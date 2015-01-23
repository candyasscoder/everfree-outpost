use core::prelude::*;
use core::cmp::{min, max};
use core::fmt;
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

    fn unfold<T, F: FnMut(Axis, T) -> (i32, T)>(val: T, mut f: F) -> (V3, T) {
        let (x, val) = f(Axis::X, val);
        let (y, val) = f(Axis::Y, val);
        let (z, val) = f(Axis::Z, val);
        (V3::new(x, y, z), val)
    }

    fn get(self, axis: Axis) -> i32 {
        match axis {
            Axis::X => self.x,
            Axis::Y => self.y,
            Axis::Z => self.z,
        }
    }

    fn fold_axes<T, F: FnMut(Axis, T) -> T>(val: T, mut f: F) -> T {
        let val = f(Axis::X, val);
        let val = f(Axis::Y, val);
        let val = f(Axis::Z, val);
        val
    }
}

pub trait Vn: Sized+Copy {
    type Axis: Eq+Copy;

    fn unfold<T, F: FnMut(<Self as Vn>::Axis, T) -> (i32, T)>(val: T, mut f: F) -> (Self, T);
    fn get(self, axis: <Self as Vn>::Axis) -> i32;
    fn fold_axes<T, F: FnMut(<Self as Vn>::Axis, T) -> T>(init: T, mut f: F) -> T;

    fn from_fn<F: FnMut(<Self as Vn>::Axis) -> i32>(mut f: F) -> Self {
        <Self as Vn>::unfold((), |a, ()| (f(a), ())).0
    }

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
        <Self as Vn>::fold_axes(0, |a, sum| sum + self.get(a) * other.get(a))
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
pub struct Region<V=V3> {
    pub min: V,
    pub max: V,
}

impl<V: Copy+fmt::Show> fmt::Show for Region<V> {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        (self.min, self.max).fmt(f)
    }
}

impl<V: Vn> Region<V> {
    #[inline]
    pub fn new(min: V, max: V) -> Region<V> {
        Region { min: min, max: max }
    }

    #[inline]
    pub fn empty(&self) -> bool {
        <V as Vn>::fold_axes(false, |a, e| e || self.min.get(a) >= self.max.get(a))
    }

    #[inline]
    pub fn points(&self) -> RegionPoints<V> {
        if self.empty() {
            RegionPoints::empty()
        } else {
            RegionPoints::new(self.min, self.max)
        }
    }

    #[inline]
    pub fn contains(&self, point: V) -> bool {
        <V as Vn>::fold_axes(true, |a, cur| {
            cur &&
            point.get(a) >= self.min.get(a) &&
            point.get(a) <  self.max.get(a)
        })
    }

    #[inline]
    pub fn contains_inclusive(&self, point: V) -> bool {
        <V as Vn>::fold_axes(true, |a, cur| {
            cur &&
            point.get(a) >= self.min.get(a) &&
            point.get(a) <= self.max.get(a)
        })
    }

    #[inline]
    pub fn join(&self, other: Region<V>) -> Region<V> {
        Region::new(self.min.zip(other.min, |a, b| min(a, b)),
                    self.max.zip(other.max, |a, b| max(a, b)))
    }

    #[inline]
    pub fn intersect(&self, other: Region<V>) -> Region<V> {
        Region::new(self.min.zip(other.min, |a, b| max(a, b)),
                    self.max.zip(other.max, |a, b| min(a, b)))
    }

    #[inline]
    pub fn overlaps(&self, other: Region<V>) -> bool {
        let inter = self.intersect(other);
        <V as Vn>::fold_axes(true, |a, over| over && inter.min.get(a) < inter.max.get(a))
    }

    #[inline]
    pub fn clamp_point(&self, point: V) -> V {
        <V as Vn>::from_fn(|a| max(self.min.get(a),
                               min(self.max.get(a),
                                   point.get(a))))
    }
}

impl<V: Vn + Add<V, Output=V> + Sub<V, Output=V> > Region<V> {
    #[inline]
    pub fn around(center: V, radius: i32) -> Region<V> {
        Region::new(center - scalar(radius),
                    center + scalar(radius))
    }

    #[inline]
    pub fn size(&self) -> V {
        self.max - self.min
    }

    #[inline]
    pub fn volume(&self) -> i32 {
        let size = self.size();
        <V as Vn>::fold_axes(1, |a, v| v * size.get(a))
    }

    #[inline]
    pub fn expand(&self, amount: V) -> Region<V> {
        Region::new(self.min - amount, self.max + amount)
    }

    #[inline]
    pub fn index(&self, point: V) -> usize {
        let offset = point - self.min;
        let size = self.size();
        <V as Vn>::fold_axes((0, 1), |a, (sum, mul)| {
            (sum + offset.get(a) as usize * mul,
             mul * size.get(a) as usize)
        }).0
    }
}

impl<V > Region<V>
        where V: Vn + Add<V, Output=V> + Sub<V, Output=V> + Div<V, Output=V> {
    #[inline]
    pub fn div_round(&self, rhs: i32) -> Region<V> {
        Region::new(self.min / scalar(rhs),
                    (self.max + scalar(rhs - 1)) / scalar(rhs))
    }

    #[inline]
    pub fn div_round_signed(&self, rhs: i32) -> Region<V> {
        Region::new(self.min.div_floor(scalar(rhs)),
                    (self.max + scalar(rhs - 1)).div_floor(scalar(rhs)))
    }
}

impl Region<V3> {
    #[inline]
    pub fn with_zs(&self, min_z: i32, max_z: i32) -> Region<V3> {
        Region::new(self.min.with_z(min_z), self.max.with_z(max_z))
    }

    #[inline]
    pub fn flatten(&self, depth: i32) -> Region<V3> {
        self.with_zs(self.min.z, self.min.z + depth)
    }
}

macro_rules! impl_Region_binop {
    ($op:ident, $method:ident) => {
        impl<V: Vn+Copy> $op<V> for Region<V> {
            type Output = Region<V>;
            fn $method(self, other: V) -> Region<V> {
                Region::new(<V as Vn>::$method(self.min, other),
                            <V as Vn>::$method(self.max, other))
            }
        }
    };
}

macro_rules! impl_Region_unop {
    ($op:ident, $method:ident) => {
        impl<V: Vn+Copy> $op for Region<V> {
            type Output = Region<V>;
            fn $method(self) -> Region<V> {
                Region::new(<V as Vn>::$method(self.min),
                            <V as Vn>::$method(self.max))
            }
        }
    };
}

macro_rules! impl_Region_shift_op {
    ($op:ident, $method:ident) => {
        impl<V: Vn+Copy> $op<usize> for Region<V> {
            type Output = Region<V>;
            fn $method(self, amount: usize) -> Region<V> {
                Region::new(<V as Vn>::$method(self.min, amount),
                            <V as Vn>::$method(self.max, amount))
            }
        }
    };
}

macro_rules! impl_Region_ops {
    () => {
        impl_Region_binop!(Add, add);
        impl_Region_binop!(Sub, sub);
        impl_Region_binop!(Mul, mul);
        impl_Region_binop!(Div, div);
        impl_Region_binop!(Rem, rem);
        impl_Region_unop!(Neg, neg);

        impl_Region_shift_op!(Shl, shl);
        impl_Region_shift_op!(Shr, shr);

        impl_Region_binop!(BitAnd, bitand);
        impl_Region_binop!(BitOr, bitor);
        impl_Region_binop!(BitXor, bitxor);
        impl_Region_unop!(Not, not);
    };
}

impl_Region_ops!();


#[derive(Copy)]
pub struct RegionPoints<V> {
    cur: V,
    min: V,
    max: V,
}

impl<V: Vn> RegionPoints<V> {
    pub fn empty() -> RegionPoints<V> {
        RegionPoints {
            cur: scalar(0),
            min: scalar(0),
            max: scalar(0),
        }
    }

    pub fn new(min: V, max: V) -> RegionPoints<V> {
        let mut first = true;
        let start = min.map(|x| {
            if first {
                first = false;
                x - 1
            } else {
                x
            }
        });
        RegionPoints {
            cur: start,
            min: min,
            max: max,
        }
    }
}

impl<V: Vn+Copy> Iterator for RegionPoints<V> {
    type Item = V;

    fn next(&mut self) -> Option<V> {
        let (new, carry) = <V as Vn>::unfold(true, |a, carry| {
            if !carry {
                (self.cur.get(a), false)
            } else {
                let new_val = self.cur.get(a) + 1;
                if new_val < self.max.get(a) {
                    (new_val, false)
                } else {
                    (self.min.get(a), true)
                }
            }
        });
        self.cur = new;
        if carry {
            None
        } else {
            Some(new)
        }
    }
}
