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

#[export_name = "collide_ramp"]
pub extern fn collide_ramp_wrapper(input: &CollideArgs, output: &mut CollideResult) {
    *output = collide_ramp(input.pos, input.size, input.velocity);
}

pub struct IsOnRampArgs {
    pos: V3,
    size: V3,
}

#[export_name = "get_ramp_angle"]
pub extern fn get_ramp_angle_wrapper(input: &IsOnRampArgs, output: &mut i32) {
    *output = get_ramp_angle(input.pos, input.size) as i32;
}

#[export_name = "get_next_ramp_angle"]
pub extern fn get_next_ramp_angle_wrapper(input: &CollideArgs, output: &mut i32) {
    *output = get_next_ramp_angle(input.pos, input.size, input.velocity) as i32;
}


#[deriving(Eq, PartialEq)]
#[repr(u8)]
pub enum Shape {
    Empty = 0,
    Floor = 1,
    Solid = 2,
    RampE = 3,
    RampW = 4,
    RampS = 5,
    RampN = 6,
    RampTop = 7,
}

#[deriving(Eq, PartialEq)]
#[repr(i32)]
pub enum RampAngle {
    NoRamp = 0,
    Flat = 1,
    XPos = 2,
    XNeg = 3,
    YPos = 4,
    YNeg = 5,
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

fn get_shape_below(mut pos: V3) -> (Shape, i32) {
    while pos.z >= 0 {
        match get_shape(pos) {
            Empty | RampTop => {},
            s => return (s, pos.z),
        }
        pos.z -= 1;
    }
    (Empty, 0)
}

impl Shape {
    fn is_ramp(&self) -> bool {
        match *self {
            RampE | RampW | RampS | RampN => true,
            _ => false,
        }
    }

    fn ramp_angle(&self) -> RampAngle {
        match *self {
            RampE => XPos,
            RampW => XNeg,
            RampS => YPos,
            RampN => YNeg,
            _ => NoRamp,
        }
    }

    fn ramp_entry_dir(&self) -> V3 {
        match *self {
            RampE => V3::new( 1,  0,  0),
            RampW => V3::new(-1,  0,  0),
            RampS => V3::new( 0,  1,  0),
            RampN => V3::new( 0, -1,  0),
            _ => fail!(),
        }
    }
}


extern {
    fn trace_ints(ptr: *const i32, len: i32);
    fn phys_trace(x: i32, y: i32, z: i32);
    fn reset_phys_trace();
}

fn trace(p: V3) {
    unsafe { phys_trace(p.x, p.y, p.z) };
}

fn reset_trace() {
    unsafe { reset_phys_trace() };
}

fn log(i: i32) {
    unsafe { trace_ints(&i as *const i32, 1) };
}

fn log_v3(v: V3) {
    log_arr(&[v.x, v.y, v.z]);
}

fn log_arr(ints: &[i32]) {
    unsafe { trace_ints(ints.as_ptr(), ints.len() as i32) };
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
    Timeout = 6,
    RampEntry = 7,
    RampExit = 8,
    RampDysfunction = 9,
    RampAngleChange = 10,
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
    let corner = pos + side * size;

    for (time, cur, hit) in PlaneCollisions::new(corner, velocity).take(3 * CHUNK_SIZE as uint) {
        let base = cur - side * size;

        let bounds = hit_chunk_boundaries(cur, hit, side);
        if (bounds != 0) {
            return CollideResult::new(base, time, bounds, ChunkBorder);
        }

        for (min, max, dir) in ContactPlanes::new(base, size, velocity.signum(), hit) {
            let mut seen_ramp = NoRamp;
            let mut seen_floor = false;

            let collided = |&:reason| {
                CollideResult::new(base, time, bits_from_hit(dir.abs()), reason)
            };

            let min_z = min.z / TILE_SIZE;

            for pos in plane_side(min, max, dir) {
                let shape = get_shape(pos);
                if pos.z == min_z {
                    match shape {
                        Floor => { seen_floor = true; },
                        RampTop => { seen_ramp = Flat; },
                        Empty => return collided(NoFloor),
                        s if s.is_ramp() && dir == s.ramp_entry_dir() => {
                            seen_ramp = s.ramp_angle();
                        },
                        _ => return collided(Wall),
                    }
                } else {
                    match shape {
                        Empty | RampTop => {},
                        _ => return collided(Wall),
                    }
                }
            }

            if seen_ramp != NoRamp {
                if !seen_floor {
                    return collided(RampEntry);
                } else {
                    return collided(Wall)
                }
            }
        }
    }

    CollideResult {
        pos: pos,
        time: 0,    // TODO: should set
        dirs: 0,
        reason: Timeout,
    }
}

fn hit_chunk_boundaries(cur: V3, hit: V3, side: V3) -> i32 {
    let chunk_side = side * scalar(CHUNK_SIZE * TILE_SIZE);
    let bound_x = hit.x != 0 && cur.x == chunk_side.x;
    let bound_y = hit.y != 0 && cur.y == chunk_side.y;
    let bound_z = hit.z != 0 && cur.z == chunk_side.z;
    
    (bound_x as i32 << 2) | (bound_y as i32 << 1) | (bound_z as i32)
}


fn collide_ramp(pos: V3, size: V3, velocity: V3) -> CollideResult {
    if velocity == scalar(0) {
        return CollideResult {
            pos: pos,
            time: 0,
            dirs: 0,
            reason: ZeroVelocity,
        }
    }

    // When moving on a ramp:
    //  - When moving downward, detect moving OUT of ramp, and indicate RampExit
    //  - When moving upward, detect moving IN to the flat region, and indicate RampAngleChange
    // When moving on a flat region (ramp top):
    //  - Detect moving OUT of the ramp, and indicate RampExit
    //  - Detect moving OUT of the flat region, and indicate RampAngleChange

    let downward = velocity.z < 0;
    let on_flat = velocity.z == 0;

    // True if we're watching for moving OUT of a region.
    let watch_out = downward || on_flat;
    let out_sign = if downward || on_flat { scalar(-1) } else { scalar(1) };

    let side = if !watch_out { velocity.is_positive() } else { velocity.is_negative() };
    let corner = pos + side * size;
    let velocity_sign = velocity.signum();

    for (time, cur, hit) in PlaneCollisions::new(corner, velocity).take(3 * CHUNK_SIZE as uint) {
        let base = cur - side * size;
        let hit = hit * V3::new(1, 1, 0);
        for (min, mut max, dir) in ContactPlanes::new(base, size, velocity_sign * out_sign, hit) {
            let collided = |&:reason| {
                CollideResult::new(base, time, bits_from_hit(dir.abs()), reason)
            };

            max.z = min.z + 1;

            if !on_flat {
                let mut any_ramp = false;
                let mut all_ramp = true;
                for pos in plane_side(min, max, velocity_sign) {
                    let (shape, _) = get_shape_below(pos);
                    if !shape.is_ramp() {
                        all_ramp = false;
                    } else {
                        any_ramp = true;
                    }
                }

                if downward && !any_ramp {
                    return collided(RampExit);
                } else if !downward && !all_ramp {
                    return collided(RampAngleChange);
                }
            } else {
                // TODO: on_flat handling is pretty hacky at the moment.
                match get_next_ramp_angle(base, size, velocity) {
                    NoRamp => return collided(RampExit),
                    Flat => {},
                    _ => return collided(RampAngleChange),
                }
            }
        }
    }

    CollideResult {
        pos: pos,
        time: 0,
        dirs: 0,
        reason: Timeout,
    }
}

// Get the ramp angle below a region.
//  - NoRamp if over only Floor, or over tiles other than Floor or Ramp
//  - Flat if over some Floor, with some Ramp at a lower z-level
//  - XPos/YPos/etc if over Ramp, possibly with some Floor at a lower z-level
fn get_ramp_angle(pos: V3, size: V3) -> RampAngle {
    let mut top_ramp_z = -1;
    let mut top_ramp = NoRamp;
    let mut top_floor_z = -1;

    for pos in plane_side(pos, pos + size * V3::new(1, 1, 0), V3::new(0, 0, 1)) {
        match get_shape_below(pos) {
            (Floor, z) => {
                if z > top_floor_z {
                    top_floor_z = z;
                }
            },
            (s, z) if s.is_ramp() => {
                if z > top_ramp_z {
                    top_ramp_z = z;
                    top_ramp = s.ramp_angle();
                }
            },
            _ => return NoRamp,
        }
    }

    if top_floor_z > top_ramp_z {
        if top_ramp == NoRamp {
            NoRamp
        } else {
            Flat
        }
    } else {
        top_ramp
    }
}

// Get the ramp angle opposite the plane in the direction of travel.  This should be used only when
// the region defined by `pos` and `size` is adjacent to a ramp entry in the direction of travel.
fn get_next_ramp_angle(pos: V3, size: V3, velocity: V3) -> RampAngle {
    // TODO: this could probably be made more efficient.
    get_ramp_angle(pos + velocity.signum(), size)
}



struct PlaneCollisions {
    units: i32,
    start: V3,
    velocity: V3,
    next: V3,
    pixel_time: V3,
}

impl PlaneCollisions {
    fn new(start: V3, velocity: V3) -> PlaneCollisions {
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

        PlaneCollisions {
            units: units,
            start: start,
            velocity: velocity,
            next: next,
            pixel_time: pixel_time,
        }
    }
}

impl Iterator<(i32, V3, V3)> for PlaneCollisions {
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

fn bits_from_hit(hit: V3) -> i32 {
    (hit.x << 2) | (hit.y << 1) | hit.z
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


struct ContactPlanes {
    box_min: V3,
    box_max: V3,
    facing: V3,
    dir_signs: V3,
    hits: HitComboIter,
}

impl ContactPlanes {
    fn new(base: V3, size: V3, dir_signs: V3, hit: V3) -> ContactPlanes {
        ContactPlanes {
            box_min: base,
            box_max: base + size,
            facing: base + size * dir_signs.is_positive(),
            dir_signs: dir_signs,
            hits: HitComboIter::new(hit),
        }
    }
}

impl Iterator<(V3, V3, V3)> for ContactPlanes {
    fn next(&mut self) -> Option<(V3, V3, V3)> {
        let cur_hit = match self.hits.next() {
            None => return None,
            Some((h, _)) => h,
        };

        let min = cur_hit.choose(&self.facing, &self.box_min);
        let max = cur_hit.choose(&self.facing, &self.box_max);
        let dir = cur_hit * self.dir_signs;

        Some((min, max, dir))
    }
}


struct RegionPoints {
    cur: V3,
    min: V3,
    max: V3,
}

impl RegionPoints {
    fn new(min: V3, max: V3) -> RegionPoints {
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

// Iterate over all tiles touching one side of the plane.  `dir` points from the plane toward the
// tiles.
fn plane_side(min: V3, max: V3, dir: V3) -> RegionPoints {
    // Plane bounds in tile coordinates.
    let tile_min = min / scalar(TILE_SIZE);
    let tile_max = (max + scalar(TILE_SIZE - 1)) / scalar(TILE_SIZE);

    // Bounds of the region on the `dir` side of the plane.
    let region_min = (tile_min - dir.is_negative()).clamp(0, CHUNK_SIZE);
    let region_max = (tile_max + dir.is_positive()).clamp(0, CHUNK_SIZE);

    RegionPoints::new(region_min, region_max)
}
