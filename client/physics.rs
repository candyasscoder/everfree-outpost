#![no_std]
#![feature(globs, phase)]
#![feature(overloaded_calls, unboxed_closures)]
#![feature(lang_items)]
#![feature(macro_rules)]
#[phase(plugin, link)] extern crate core;
use core::prelude::*;
use core::cmp::{min, max};
use core::cell::Cell;
use core::iter;
use core::iter::Peekable;


macro_rules! try_return {
    ($e:expr) => {
        match $e {
            Some(x) => return x,
            None => {},
        }
    }
}

macro_rules! try_return_some {
    ($e:expr) => {
        match $e {
            Some(x) => return Some(x),
            None => {},
        }
    }
}



mod std {
    pub use core::cmp;
    pub use core::fmt;
}


#[inline(always)] #[cold]
#[lang = "fail_fmt"]
extern fn lang_fail_fmt(args: &core::fmt::Arguments,
                        file: &'static str,
                        line: uint) -> ! {
    unsafe { core::intrinsics::abort() };
}

#[inline(always)] #[cold]
#[lang = "stack_exhausted"]
extern fn lang_stack_exhausted() -> ! {
    unsafe { core::intrinsics::abort() };
}

#[inline(always)] #[cold]
#[lang = "eh_personality"]
extern fn lang_eh_personality() -> ! {
    unsafe { core::intrinsics::abort() };
}


fn gcd(mut a: i32, mut b: i32) -> i32 {
    while (b != 0) {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

fn lcm(a: i32, b: i32) -> i32 {
    a * b / gcd(a, b)
}


#[deriving(Eq, PartialEq)]
pub struct V3 {
    x: i32,
    y: i32,
    z: i32,
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

    fn abs(&self) -> V3 {
        self.map(|&:a: i32| a.abs())
    }

    fn signum(&self) -> V3 {
        self.map(|&:a: i32| a.signum())
    }

    fn is_positive(&self) -> V3 {
        self.map(|&:a: i32| (a > 0) as i32)
    }

    fn is_negative(&self) -> V3 {
        self.map(|&:a: i32| (a < 0) as i32)
    }

    fn is_zero(&self) -> V3 {
        self.map(|&:a: i32| (a == 0) as i32)
    }

    fn choose(&self, a: &V3, b: &V3) -> V3 {
        V3 {
            x: if self.x != 0 { a.x } else { b.x },
            y: if self.y != 0 { a.y } else { b.y },
            z: if self.z != 0 { a.z } else { b.z },
        }
    }

    fn clamp(&self, low: i32, high: i32) -> V3 {
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


pub struct CollideArgs {
    pos: V3,
    size: V3,
    velocity: V3,
}

#[export_name = "collide"]
pub extern fn collide_wrapper(input: &CollideArgs, output: &mut CollideResult) {
    *output = collide(input.pos, input.size, input.velocity);
}


#[repr(u8)]
pub enum Shape {
    Empty = 0,
    Floor = 1,
    Solid = 2,
    RampE = 3,
}

static TILE_SIZE: i32 = 32;
static CHUNK_SIZE: i32 = 16;
static SHAPE_BUFFER: *const Shape = 4096 as *const Shape;

fn get_shape(pos: V3) -> Shape {
    let V3 { x, y, z } = pos;
    if x < 0 || x >= CHUNK_SIZE || y < 0 || y >= CHUNK_SIZE || z < 0 || z >= CHUNK_SIZE {
        return Empty;
    }

    let index = ((z) * CHUNK_SIZE + y) * CHUNK_SIZE + x;
    unsafe { *SHAPE_BUFFER.offset(index as int) }
}


extern {
    fn phys_trace(x: i32, y: i32, z: i32);
}

fn trace(p: V3) {
    unsafe { phys_trace(p.x, p.y, p.z) };
}


pub struct CollideResult {
    pos: V3,
    time: i32,
    dirs: i32,
    reason: CollideReason,
}

impl CollideResult {
    pub fn new(pos: V3, time: i32, dirs: i32, reason: CollideReason) -> CollideResult {
        CollideResult {
            pos: pos,
            time: time,
            dirs: dirs,
            reason: reason,
        }
    }
}

#[repr(i32)]
pub enum CollideReason {
    ZeroVelocity = 1,
    NoFloor = 2,
    Wall = 3,
    SlideEnd = 4,
    ChunkBorder = 5,
    RampBottom = 6,
}



fn collide(pos: V3, size: V3, velocity: V3) -> CollideResult {
    if velocity == scalar(0) {
        return CollideResult {
            pos: pos,
            time: 0,
            dirs: 0,
            reason: ZeroVelocity,
        }
    }

    let side = velocity.is_positive();
    let dir = velocity.signum();
    let corner = pos + side * size;

    let my_check_plane = |min, max, dir| {
        let seen_ramp_bottom = Cell::new(false);
        let seen_floor = Cell::new(false);

        try_return_some!(check_plane(min, max, dir, |pos| {
            //trace(pos);
            let shape = get_shape(pos);
            if pos.z == min.z {
                match shape {
                    Floor => {
                        seen_floor.set(true);
                        None
                    },
                    RampE if dir == V3::new(1, 0, 0) => {
                        seen_ramp_bottom.set(true);
                        None
                    },
                    Empty => Some(NoFloor),
                    _ => Some(Wall),
                }
            } else {
                match shape {
                    Empty => None,
                    _ => Some(Wall),
                }
            }
        }));

        // Otherwise, check_plane returned None.
        if seen_ramp_bottom.get() {
            if seen_floor.get() {
                Some(RampBottom)
            } else {
                Some(Wall)
            }
        } else {
            None
        }
    };

    try_return!(walk(corner, velocity, |cur, time, hit| {
        let base = cur - side * size;

        let bounds = hit_chunk_boundaries(cur, hit, side);
        if (bounds != 0) {
            return Some(CollideResult::new(base, time, bounds, ChunkBorder));
        }

        let min = base / scalar(TILE_SIZE);
        let max = (base + size + scalar(TILE_SIZE - 1)) / scalar(TILE_SIZE);
        let facing = cur / scalar(TILE_SIZE);

        for (cur_hit, dirs) in HitComboIter::new(hit) {
            //trace(cur_hit);
            //trace(scalar(dirs));
            match my_check_plane(cur_hit.choose(&facing, &min),
                                 cur_hit.choose(&facing, &max),
                                 dir * cur_hit) {
                Some(reason) => return Some(CollideResult::new(base, time, dirs, reason)),
                None => {},
            }
        }

        None
    }));

    CollideResult {
        pos: pos,
        time: 0,
        dirs: 0,
        reason: ZeroVelocity,
    }
}

fn check_plane(min: V3, max: V3, dir: V3, callback: |V3| -> Option<CollideReason>) -> Option<CollideReason> {
    let min = (min - dir.is_negative()).clamp(0, CHUNK_SIZE);
    let max = (max + dir.is_positive()).clamp(0, CHUNK_SIZE);

    for z in range(min.z, max.z) {
        for y in range(min.y, max.y) {
            for x in range(min.x, max.x) {
                try_return_some!(callback(V3::new(x, y, z)));
            }
        }
    }

    None
}

fn hit_chunk_boundaries(cur: V3, hit: V3, side: V3) -> i32 {
    let chunk_side = side * scalar(CHUNK_SIZE * TILE_SIZE);
    let bound_x = hit.x != 0 && cur.x == chunk_side.x;
    let bound_y = hit.y != 0 && cur.y == chunk_side.y;
    let bound_z = hit.z != 0 && cur.z == chunk_side.z;
    
    (bound_x as i32 << 2) | (bound_y as i32 << 1) | (bound_z as i32)
}


struct CollisionIter {
    units: i32,
    start: V3,
    velocity: V3,
    next: V3,
    pixel_time: V3,
}

impl CollisionIter {
    fn new(start: V3, velocity: V3) -> CollisionIter {
        assert!(velocity != scalar(0));

        // We subdivide both time and space into `u` subpixels and `u` timesteps per second.  The
        // result is that all interesting events occur at an integer number of subpixels and timesteps.
        let units =
            lcm(if velocity.x != 0 { velocity.x.abs() } else { 1 },
            lcm(if velocity.y != 0 { velocity.y.abs() } else { 1 },
                if velocity.z != 0 { velocity.z.abs() } else { 1 }));

        // Find the coordinates of the first plane we will hit in each direction.
        let side = velocity.is_positive();
        let first_plane =
            (start + side * scalar(TILE_SIZE - 1)) / scalar(TILE_SIZE) * scalar(TILE_SIZE);

        // For each axis, the time (in `1/u`-second timesteps) to move one pixel.
        let pixel_time = velocity.map(|&: a: i32| if a != 0 { units / a.abs() } else { 0 });

        // For each axis, the timestapm of the next collision.
        let next = pixel_time.zip(&(first_plane - start).abs(),
            |&: p: i32, d: i32| if p != 0 { p * d } else { core::i32::MAX });

        CollisionIter {
            units: units,
            start: start,
            velocity: velocity,
            next: next,
            pixel_time: pixel_time,
        }
    }
}

impl Iterator<(i32, V3, V3)> for CollisionIter {
    #[inline]
    fn next(&mut self) -> Option<(i32, V3, V3)> {
        let time = min(self.next.x, min(self.next.y, self.next.z));
        // Check which axes have a collision (may be more than one).
        let hit = self.next.map(|&: a: i32| (a == time) as i32);
        // Advance the next collision time by `pixel_time * TILE_SIZE` steps for each axis that is
        // currently colliding.
        self.next = self.next + hit * self.pixel_time * scalar(TILE_SIZE);

        let cur_pos = self.start + self.velocity * scalar(time) / scalar(self.units);
        let time_ms = 1000 * time / self.units;

        Some((time_ms, cur_pos, hit))
    }
}

fn walk(start: V3, velocity: V3,
        callback: |V3, i32, V3| -> Option<CollideResult>) -> Option<CollideResult> {
    for (time, pos, hit) in CollisionIter::new(start, velocity).take(3 * CHUNK_SIZE as uint) {
        try_return_some!(callback(pos, time, hit));
    }
    None
}


static HIT_COMBO_ORDER: u32 = 0b111_110_011_101_100_010_001_000;

struct HitComboIter {
    cur: u32,
    mask: u8,
}

impl HitComboIter {
    fn new(hit: V3) -> HitComboIter {
        HitComboIter {
            cur: HIT_COMBO_ORDER,
            mask: ((hit.x << 2) | (hit.y << 1) | (hit.z)) as u8,
        }
    }
}

impl Iterator<(V3, i32)> for HitComboIter {
    #[inline]
    fn next(&mut self) -> Option<(V3, i32)> {
        self.cur >>= 3;
        let inv_mask = !self.mask as u32 & 0b111;
        while (self.cur & inv_mask) != 0 {
            self.cur >>= 3;
        }

        if self.cur == 0 {
            return None;
        }

        let bits = (self.cur & 0b111) as i32;
        Some((hit_from_bits(bits), bits))
    }
}

fn hit_from_bits(bits: i32) -> V3 {
    V3::new((bits >> 2) & 1,
            (bits >> 1) & 1,
            (bits) & 1)
}



struct Interleaving<A, B, I, J, F> {
    i: Peekable<(A, B), I>,
    j: Peekable<(A, B), J>,
    combine: F,
}

impl<A, B, I, J, F> Interleaving<A, B, I, J, F>
        where I: Iterator<(A, B)>,
              J: Iterator<(A, B)>,
              A: Ord,
              F: FnMut(B, B) -> B {
    fn new(i: I, j: J, combine: F) -> Interleaving<A, B, I, J, F> {
        Interleaving {
            i: i.peekable(),
            j: j.peekable(),
            combine: combine,
        }
    }
}

impl<A, B, I, J, F> Iterator<(A, B)> for Interleaving<A, B, I, J, F>
        where I: Iterator<(A, B)>,
              J: Iterator<(A, B)>,
              A: Ord,
              F: FnMut(B, B) -> B {
    #[inline]
    fn next(&mut self) -> Option<(A, B)> {
        let ordering = match (self.i.peek(), self.j.peek()) {
            (Some(&(ref t1, _)), Some(&(ref t2, _))) => t1.cmp(t2),
            (Some(_), None) => Less,
            (None, Some(_)) => Greater,
            (None, None) => return None,
        };

        match ordering {
            Less => self.i.next(),
            Greater => self.j.next(),
            Equal => {
                let (t, a) = self.i.next().unwrap();
                let (_, b) = self.i.next().unwrap();
                Some((t, (self.combine)(a, b)))
            },
        }
    }
}
