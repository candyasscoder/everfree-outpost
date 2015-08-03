//! Diamond-Square with Constraints
//!
//! Operates like regular diamond-square, but the grid can be initialized with constraints on the
//! values of cells.  (Example: "the cell at (3, 5) should have a value between 10 and 20.")
//! Random choices made by the algorithm will be biased such that the final configuration satisfies
//! as many constraints as possible.

use std::cmp;
use std::iter;
use std::ops::{Add, Sub, Div};
use std::u8;
use std::u32;
use rand::Rng;

use types::*;

use util::small_vec::SmallVec;

pub use self::Phase::{Diamond, Square};


/// Wrapper around `u32` for performing fixed-point arithmetic.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
struct Fixed(u32);

/// Number of fractional bits used by `Fixed`.  This is chosen to allow adding up four values in
/// the range 0.0 <= x < 256.0, without overflow.
const FIXEDPOINT_BASE: u8 = 32 - (8 + 2);

impl Fixed {
    /// Convert a `u8` to a `Fixed` in the range 0..256.
    fn from_u8(x: u8) -> Fixed {
        Fixed((x as u32) << FIXEDPOINT_BASE)
    }

    fn from_u8_mid(x: u8) -> Fixed {
        let extra = 1 << (FIXEDPOINT_BASE - 1);
        Fixed::from_u8(x) + Fixed(extra)
    }

    /// Convert a `u8` to the largest `Fixed` that truncates to the same `u8` value.  (That is,
    /// convert `x` to `x + 0.999...`.
    fn from_u8_max(x: u8) -> Fixed {
        Fixed(((x as u32 + 1) << FIXEDPOINT_BASE) - 1)
    }

    /// Get the maximum `Fixed` that truncates to a `u8` without overflow.
    fn max_u8() -> Fixed {
        Fixed::from_u8_max(u8::MAX)
    }

    /// Truncate a `Fixed` to a `u8`.
    fn as_u8(self) -> u8 {
        (self.0 >> FIXEDPOINT_BASE) as u8
    }

    /// Round up to the next higher integer, then truncate to `u8`.
    fn as_u8_ceil(self) -> u8 {
        (self + Fixed::from_u8_max(0)).clamp_u8().as_u8()
    }

    /// Extract the raw `u32` representation.
    fn unwrap(self) -> u32 {
        self.0
    }

    /// Clamp to a value that truncates to `u8` without overflow.
    fn clamp_u8(self) -> Fixed {
        if self > Fixed::max_u8() {
            Fixed::max_u8()
        } else {
            self
        }
    }

    /// Saturating addition in the range 0..256.
    fn add_sat_u8(self, other: Fixed) -> Fixed {
        let val = self.0.saturating_add(other.0);
        let max = Fixed::max_u8().unwrap();
        if val > max {
            Fixed(max)
        } else {
            Fixed(val)
        }
    }

    /// Saturating subtraction in the range 0..256.
    fn sub_sat_u8(self, other: Fixed) -> Fixed {
        Fixed(self.0.saturating_sub(other.0))
    }

    /// Multiply by `mul` and then divide by `div`, rounding only at the end.  Uses `u64`
    /// arithmetic internally to avoid overflow.
    fn mul_div(self, mul: Fixed, div: Fixed) -> Fixed {
        let x = self.0 as u64;
        let m = mul.0 as u64;
        let d = div.0 as u64;
        let r = x * m / d;
        if r > u32::MAX as u64 {
            panic!("arithmetic overflow in mul_div");
        } else {
            Fixed(r as u32)
        }
    }
}

impl Add<Fixed> for Fixed {
    type Output = Fixed;
    fn add(self, other: Fixed) -> Fixed {
        Fixed(self.0 + other.0)
    }
}

impl Sub<Fixed> for Fixed {
    type Output = Fixed;
    fn sub(self, other: Fixed) -> Fixed {
        Fixed(self.0 - other.0)
    }
}

impl Div<u32> for Fixed {
    type Output = Fixed;
    fn div(self, other: u32) -> Fixed {
        Fixed(self.0 / other)
    }
}



#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    Diamond,
    Square,
}

bitflags! {
    flags PointFlags: u8 {
        /// The range for the cell has been set externally.
        const PRESET_RANGE =            0x01,
        #[doc = "There is a constraint on the cell."]
        const HAS_CONSTRAINT =          0x02,
        #[doc = "There is a constraint on the cell or on another cell whose value depends on the \
        value of this one."]
        const CHILD_HAS_CONSTRAINT =    0x04,
        /// The `value` field for the cell has been initialized.
        const HAS_VALUE =               0x08,
    }
}



const MAX_WEIGHT: Fixed = Fixed(0x100 << FIXEDPOINT_BASE);


/// Compute the offsets and distance for the four surrounding points that should contribute to each
/// value computed at the given level and phase.  For example, if `phase` is `Diamond`, it produces
/// a `V2` in each cardinal direction, and a distance scaled based on `level`.
fn parent_dirs(level: u8, phase: Phase) -> (&'static [V2; 4], i32) {
    static DIAMOND_PARENT_DIRS: [V2; 4] = [
        V2 { x:  1, y:  0 },
        V2 { x:  0, y:  1 },
        V2 { x: -1, y:  0 },
        V2 { x:  0, y: -1 },
    ];

    static SQUARE_PARENT_DIRS: [V2; 4] = [
        V2 { x:  1, y:  1 },
        V2 { x: -1, y:  1 },
        V2 { x:  1, y: -1 },
        V2 { x: -1, y: -1 },
    ];

    let dirs = match phase {
        Diamond => &DIAMOND_PARENT_DIRS,
        Square => &SQUARE_PARENT_DIRS,
    };
    let dist = 1 << level;
    (dirs, dist)
}

pub struct DscGrid<F> {
    /// The value of each cell.  Only valid for cells that have the `PRESET_VALUE` flag set.
    value: Box<[Fixed]>,
    /// The range of possible values for each cell.  Only valid for cells that have the
    /// `CHILD_HAS_CONSTRAINT` flag set.
    range: Box<[(Fixed, Fixed)]>,
    /// The constraint on each cell.  Only valid for cells with `HAS_CONSTRAINT` set.
    constraint: Box<[(Fixed, Fixed)]>,
    /// List of cells that have constraints.
    constrained_points: Vec<V2>,
    /// The flags for each cell.
    flags: Box<[PointFlags]>,
    /// The degree to which the target cell's value affects each other cell.  This is a temporary,
    /// only valid within `fill_one_constrained` after the call to `calc_weight`, and only for
    /// cells with `CHILD_HAS_CONSTRAINT` set.  
    weight: Box<[Fixed]>,
    /// The size of the grid.  Points within `0 <= x <= size.x && 0 <= y <= size.y` lie within the
    /// grid.  (Note the `<=` on the upper bound comparison.)
    size: V2,
    /// Number of levels of subdivision below the seed points.  That is, seed points are spaced on
    /// a `1 << seed_level` unit grid.
    seed_level: u8,
    /// Closure to compute the maximum amount of random offset that can be applied to a cell.  The
    /// actual offset will be in the range `-max <= actual <= max`.
    get_max_offset: F,
}

// TODO: There's a lot of potential for rounding errors in here.  I haven't really checked to make
// sure everything lines up, so there may be situations where (e.g.) filling a grid, erasing some
// values (leaving the rest as constraints), and regenerating may fail to produce a valid grid.

impl<F> DscGrid<F> {
    pub fn new(size: V2, seed_level: u8, get_max_offset: F) -> DscGrid<F> {
        let len = ((size.x + 1) * (size.y + 1)) as usize;
        let f0 = Fixed::from_u8(0);
        let value = iter::repeat(f0).take(len).collect::<Vec<_>>().into_boxed_slice();
        let range = iter::repeat((f0, f0)).take(len).collect::<Vec<_>>().into_boxed_slice();
        let constraint = iter::repeat((f0, f0)).take(len).collect::<Vec<_>>().into_boxed_slice();
        let flags = iter::repeat(PointFlags::empty()).take(len).collect::<Vec<_>>()
                                                     .into_boxed_slice();
        let weight = iter::repeat(f0).take(len).collect::<Vec<_>>().into_boxed_slice();

        DscGrid {
            value: value,
            range: range,
            constraint: constraint,
            constrained_points: Vec::new(),
            flags: flags,
            weight: weight,
            size: size,
            seed_level: seed_level,
            get_max_offset: get_max_offset,
        }
    }


    pub fn debug(&self) {
        fn dump<F: FnMut(V2) -> String>(size: V2, mut f: F) {
            for y in 0 .. size.y + 1 {
                let mut line = String::new();
                for x in 0 .. size.x + 1 {
                    line.push_str(&*f(V2::new(x, y)));
                }
                debug!("{}", line);
            }
        }

        dump(self.size, |p| {
            let idx = self.bounds().index(p);
            format!("{:3} ", self.value[idx].as_u8())
        });
    }


    /// Produce a region that contains every point in the grid.
    pub fn bounds(&self) -> Region<V2> {
        Region::new(scalar(0), self.size + scalar(1))
    }

    fn set_value(&mut self, pos: V2, value: Fixed) {
        if !self.bounds().contains(pos) {
            return;
        }

        let idx = self.bounds().index(pos);
        self.flags[idx].insert(HAS_VALUE);
        self.value[idx] = value;
        self.range[idx] = (value, value);
    }

    pub fn get_value(&self, pos: V2) -> Option<u8> {
        if !self.bounds().contains(pos) {
            return None;
        }

        let idx = self.bounds().index(pos);
        if !self.flags[idx].contains(HAS_VALUE) {
            return None;
        }

        Some(self.value[idx].as_u8())
    }

    /// Set the range of allowable values for a seed point.  The value for that cell will be chosen
    /// from the range `low <= x <= high`, possibly biased due to constraints.
    pub fn set_range(&mut self, pos: V2, low: u8, high: u8) {
        if !self.bounds().contains(pos) {
            return;
        }

        let idx = self.bounds().index(pos);
        self.flags[idx].insert(PRESET_RANGE);
        self.range[idx] = (Fixed::from_u8(low),
                           Fixed::from_u8_max(high));
    }

    pub fn get_range(&self, pos: V2) -> Option<(u8, u8)> {
        if !self.bounds().contains(pos) {
            return None;
        }

        let idx = self.bounds().index(pos);
        if !self.flags[idx].contains(PRESET_RANGE) {
            return None;
        }

        let (min, max) = self.range[idx];
        Some((min.as_u8(), max.as_u8()))
    }

    pub fn set_constrained(&mut self, pos: V2) {
        if !self.bounds().contains(pos) {
            return;
        }

        let idx = self.bounds().index(pos);
        if self.flags[idx].contains(PRESET_RANGE) {
            self.flags[idx].insert(HAS_CONSTRAINT);
            self.constraint[idx] = self.range[idx];
            self.constrained_points.push(pos);
        }
    }
}

impl<F> DscGrid<F>
        where F: FnMut(V2, u8, Phase) -> u8 {
    fn calc_child_has_constraint(&mut self) {
        // Bottom-up traversal
        let bounds = self.bounds();

        for level in 0 .. self.seed_level {
            let step = 1 << (level + 1);
            let half = 1 << level;

            // Diamond
            for p in (bounds / scalar(step)).points_inclusive() {
                let p = p * scalar(step);
                let north = p + V2::new(half, 0);
                let west = p + V2::new(0, half);
                if bounds.contains(north) {
                    self.calc_one_child_has_constraint(north, level, Diamond);
                }
                if bounds.contains(west) {
                    self.calc_one_child_has_constraint(west, level, Diamond);
                }
            }

            // Square
            for p in (bounds / scalar(step)).points() {
                let p = p * scalar(step);
                let center = p + scalar(half);
                if bounds.contains(center) {
                    self.calc_one_child_has_constraint(center, level, Square);
                }
            }
        }
    }

    fn calc_one_child_has_constraint(&mut self, pos: V2, level: u8, phase: Phase) {
        let bounds = self.bounds();
        let idx = bounds.index(pos);
        if self.flags[idx].contains(HAS_CONSTRAINT) {
            self.flags[idx].insert(CHILD_HAS_CONSTRAINT);
        }
        if !self.flags[idx].contains(CHILD_HAS_CONSTRAINT) {
            return;
        }
        // Otherwise, the flag is set here, so propagate it to parents.

        let (dirs, dist) = parent_dirs(level, phase);
        for &d in dirs.iter() {
            let p = pos + d * scalar(dist);
            if !bounds.contains(p) {
                continue;
            }

            self.flags[bounds.index(p)].insert(CHILD_HAS_CONSTRAINT);
        }
    }


    fn calc_range(&mut self) {
        // Before running this function, initialize ranges for all seed points.
        let bounds = self.bounds();

        for level in (0 .. self.seed_level).rev() {
            let step = 1 << (level + 1);
            let half = 1 << level;

            // Square
            for base in (bounds / scalar(step)).points() {
                let base = base * scalar(step);
                let center = base + scalar(half);
                if bounds.contains(center) {
                    self.calc_one_range(center, level, Square);
                }
            }

            // Diamond
            for base in (bounds / scalar(step)).points_inclusive() {
                let base = base * scalar(step);
                let north = base + V2::new(half, 0);
                let west = base + V2::new(0, half);
                if bounds.contains(north) {
                    self.calc_one_range(north, level, Diamond);
                }
                if bounds.contains(west) {
                    self.calc_one_range(west, level, Diamond);
                }
            }
        }
    }

    fn calc_one_range(&mut self, pos: V2, level: u8, phase: Phase) {
        let bounds = self.bounds();
        let idx = bounds.index(pos);
        if !self.flags[idx].contains(CHILD_HAS_CONSTRAINT) {
            return;
        }
        if self.flags[idx].contains(PRESET_RANGE) {
            return;
        }

        let (dirs, dist) = parent_dirs(level, phase);
        let mut min_sum = Fixed::from_u8(0);
        let mut max_sum = Fixed::from_u8(0);
        let mut count = 0;
        for &d in dirs.iter() {
            let p = pos + d * scalar(dist);
            if !bounds.contains(p) {
                continue;
            }

            let (min, max) = self.range[bounds.index(p)];
            min_sum = min_sum + min;
            max_sum = max_sum + max;
            count += 1;
        }

        let offset = Fixed::from_u8((self.get_max_offset)(pos, level, phase));

        self.range[idx] = (min_sum / count - offset,
                           max_sum / count + offset);
    }


    fn calc_weight(&mut self, src: V2, init_level: u8, init_phase: Phase) {
        // Before running this function, initialize ranges for all seed points.
        let bounds = self.bounds();

        for x in self.weight.iter_mut() {
            *x = Fixed::from_u8(0);
        }
        self.weight[bounds.index(src)] = MAX_WEIGHT;

        // We want to start with the phase *after* (init_level, init_phase).
        let init_square = false;
        let init_diamond = init_level < self.seed_level && init_phase == Square;

        for level in (0 .. init_level + 1).rev() {
            let step = 1 << (level + 1);
            let half = 1 << level;

            // Square
            if level != init_level || init_square {
                for base in (bounds / scalar(step)).points() {
                    let base = base * scalar(step);
                    let center = base + scalar(half);
                    if bounds.contains(center) {
                        self.calc_one_weight(center, level, Square);
                    }
                }
            }

            // Diamond
            if level != init_level || init_diamond {
                for base in (bounds / scalar(step)).points_inclusive() {
                    let base = base * scalar(step);
                    let north = base + V2::new(half, 0);
                    let west = base + V2::new(0, half);
                    if bounds.contains(north) {
                        self.calc_one_weight(north, level, Diamond);
                    }
                    if bounds.contains(west) {
                        self.calc_one_weight(west, level, Diamond);
                    }
                }
            }
        }
    }

    fn calc_one_weight(&mut self, pos: V2, level: u8, phase: Phase) {
        let bounds = self.bounds();
        let idx = bounds.index(pos);
        if !self.flags[idx].contains(CHILD_HAS_CONSTRAINT) {
            return;
        }

        let (dirs, dist) = parent_dirs(level, phase);
        let mut sum = Fixed::from_u8(0);
        let mut count = 0;
        for &d in dirs.iter() {
            let p = pos + d * scalar(dist);
            if !bounds.contains(p) {
                continue;
            }

            sum = sum + self.weight[bounds.index(p)];
            count += 1;
        }

        self.weight[bounds.index(pos)] = sum / count;
    }


    fn valid_range(&self,
                   target: V2) -> Option<(Fixed, Fixed)> {
        let bounds = self.bounds();
        let (target_cur_min, target_cur_max) = self.range[bounds.index(target)];
        struct Entry {
            min: Fixed,
            max: Fixed,
            count: u32,
        }
        let mut ranges = SmallVec::<Entry>::new();

        for &pos in self.constrained_points.iter() {
            let idx = bounds.index(pos);
            let (pos_cur_min, pos_cur_max) = self.range[idx];
            let (c_min, c_max) = self.constraint[idx];
            let weight = self.weight[idx];
            if weight == Fixed::from_u8(0) {
                continue;
            }

            // Here's the math:
            // `target` is about to choose a single value within `target_cur_range`.  This means it
            // will change its range from `target_cur_min .. target_cur_max` to `value .. value`.
            // In the range computation for `pos`, there is a term `weight * target_range`, and the
            // result of this computation must overlap the constraint range `c`.  That is, we want:
            //      c_min < pos_max = weight * target_value + other_max
            //      c_max > pos_min = weight * target_value + other_min
            // Rearranged:
            //      weight * target_value > c_min - other_max
            //      weight * target_value < c_max - other_min
            // Where
            //      other_max = pos_cur_max - weight * target_cur_max
            //      other_min = pos_cur_min - weight * target_cur_min
            //
            // Now, some arithmetic manipulation:
            //      target_value > (c_min - pos_cur_max + weight * target_cur_max) / weight
            //      target_value < (c_max - pos_cur_min + weight * target_cur_min) / weight
            //
            //      target_value > (c_min - pos_cur_max) / weight + target_cur_max
            //      target_value < (c_max - pos_cur_min) / weight + target_cur_min

            // We do this calculation in i64 rather than `Fixed` because it may involve negative
            // numbers.

            fn clamp_fixed_u8(x: i64) -> Fixed {
                if x < 0 {
                    Fixed(0)
                } else if x > Fixed::max_u8().unwrap() as i64 {
                    Fixed::max_u8()
                } else {
                    Fixed(x as u32)
                }
            }

            let min_tmp = c_min.unwrap() as i64 - pos_cur_max.unwrap() as i64;
            let min_tmp = min_tmp * (MAX_WEIGHT.unwrap() as i64) / (weight.unwrap() as i64);

            let max_tmp = c_max.unwrap() as i64 - pos_cur_min.unwrap() as i64;
            let max_tmp = max_tmp * (MAX_WEIGHT.unwrap() as i64) / (weight.unwrap() as i64);

            let satisfying_min = clamp_fixed_u8(min_tmp + (target_cur_max.unwrap() as i64));
            let satisfying_max = clamp_fixed_u8(max_tmp + (target_cur_min.unwrap() as i64));

            let range_min = cmp::max(satisfying_min, target_cur_min);
            let range_max = cmp::min(satisfying_max, target_cur_max);

            if range_max < range_min {
                continue;
            }

            let mut overlapped_any = false;
            for entry in ranges.iter_mut() {
                if range_min <= entry.max && entry.min <= range_max {
                    overlapped_any = true;
                    entry.min = cmp::max(entry.min, range_min);
                    entry.max = cmp::min(entry.max, range_max);
                    entry.count += 1;
                }
            }

            if !overlapped_any {
                ranges.push(Entry {
                    min: range_min,
                    max: range_max,
                    count: 1
                });
            }
        }

        ranges.iter()
              .max_by(|e| e.count)
              .map(|e| (e.min, e.max))
    }


    /// Fill in the entire grid using diamond-square.
    ///
    /// Before running this function, initialize ranges for all seed points.
    pub fn fill<R: Rng>(&mut self, rng: &mut R) {
        self.calc_child_has_constraint();
        let bounds = self.bounds();

        let level = self.seed_level;
        for seed_pos in (bounds / scalar(1 << level)).points_inclusive() {
            let pos = seed_pos * scalar(1 << level);
            self.fill_one(rng, pos, level, Diamond);
        }

        for level in (0 .. self.seed_level).rev() {
            let step = 1 << (level + 1);
            let half = 1 << level;

            // Square
            for base in (bounds / scalar(step)).points() {
                let base = base * scalar(step);
                let center = base + scalar(half);
                if bounds.contains(center) {
                    self.fill_one(rng, center, level, Square);
                }
            }

            // Diamond
            for base in (bounds / scalar(step)).points_inclusive() {
                let base = base * scalar(step);
                let north = base + V2::new(half, 0);
                let west = base + V2::new(0, half);
                if bounds.contains(north) {
                    self.fill_one(rng, north, level, Diamond);
                }
                if bounds.contains(west) {
                    self.fill_one(rng, west, level, Diamond);
                }
            }
        }
    }

    fn fill_one<R: Rng>(&mut self, rng: &mut R, pos: V2, level: u8, phase: Phase) {
        let idx = self.bounds().index(pos);
        let flags = self.flags[idx];
        if flags.contains(CHILD_HAS_CONSTRAINT) {
            self.fill_one_constrained(rng, pos, level, phase);
        } else if flags.contains(PRESET_RANGE) {
            let (min, max) = self.range[idx];
            let val = Fixed(rng.gen_range(min.unwrap(), max.unwrap() + 1));
            self.set_value(pos, val);
        } else {
            self.fill_one_random(rng, pos, level, phase);
        }
    }

    fn fill_one_constrained<R: Rng>(&mut self, rng: &mut R, pos: V2, level: u8, phase: Phase) {
        self.calc_range();
        self.calc_weight(pos, level, phase);

        let orig_range = self.range[self.bounds().index(pos)];
        let (min, max) = self.valid_range(pos).unwrap_or(orig_range);
        let val = Fixed(rng.gen_range(min.unwrap(), max.unwrap() + 1));
        self.set_value(pos, val);
    }

    fn fill_one_random<R: Rng>(&mut self, rng: &mut R, pos: V2, level: u8, phase: Phase) {
        let bounds = self.bounds();
        let idx = bounds.index(pos);

        let (dirs, dist) = parent_dirs(level, phase);
        let mut sum = Fixed::from_u8(0);
        let mut count = 0;
        for &d in dirs.iter() {
            let p = pos + d * scalar(dist);
            if !bounds.contains(p) {
                continue;
            }

            sum = sum + self.value[bounds.index(p)];
            count += 1;
        }
        let base = sum / count;
        let raw_base = base.unwrap() as i32;

        let max_offset = ((self.get_max_offset)(pos, level, phase) as i32) << FIXEDPOINT_BASE;
        let offset = rng.gen_range(-max_offset, max_offset + 1);

        let val =
            if raw_base < -offset { Fixed::from_u8(0) }
            else { Fixed((raw_base + offset) as u32).clamp_u8() };

        self.set_value(pos, val);
    }
}
