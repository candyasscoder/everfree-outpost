use rand::{Rng, XorShiftRng, SeedableRng};

use types::*;

use terrain_gen::Field;


type Level = u8;

// Use references to unboxed closures to allow recursive calls to use the same type parameters.
// (Wrapping them into new closures would produce new type parameters for each recursion depth,
// leading monomorphization to diverge.)
fn diamond_square<Check, Handle>(check: &Check,
                                 mut handle: &mut Handle,
                                 seed: u64,
                                 offsets: &[i32],
                                 base: V2,
                                 init: [i32; 4])
        where Check: Fn(Region2) -> bool,
              Handle: FnMut(V2, i32) {
    assert!(offsets.len() >= 2);
    assert!(offsets.len() % 2 == 0);

    let level = (offsets.len() / 2 - 1) as Level;

    let mut arr = [
        init[0],    0,          init[1],
        0,          0,          0,
        init[2],    0,          init[3],
    ];

    let mut r: XorShiftRng = SeedableRng::from_seed([(seed >> 32) as u32,
                                                     seed as u32 ^ level as u32,
                                                     base.x as u32,
                                                     base.y as u32]);

    for &(i, j) in [(0, 2), (0, 6), (2, 8), (6, 8)].iter() {
        let (i, j): (usize, usize) = (i, j);
        let k = (i + j) / 2;
        arr[k] = (arr[i] + arr[j]) / 2 + r.gen_range(-offsets[0], offsets[0] + 1);
    }

    arr[4] = (arr[1] + arr[3] + arr[5] + arr[7]) / 4 + r.gen_range(-offsets[1], offsets[1] + 1);

    let step = scalar(1 << level as usize);
    {
        let mut emit = |dx, dy| {
            let idx = (dx + 3 * dy) as usize;
            (*handle)(base + step * V2::new(dx, dy), arr[idx]);
        };
        emit(1, 0);
        emit(0, 1);
        emit(1, 2);
        emit(2, 1);
        emit(1, 1);
    }
    if level >= 1 {
        let new_offsets = &offsets[2..];
        for d in Region2::new(scalar(0), scalar(2)).points() {
            let new_base = base + step * d;
            if (*check)(Region2::new(new_base, new_base + step)) {
                let idx = (d.x + 3 * d.y) as usize;
                diamond_square(check,
                               handle,
                               seed,
                               new_offsets,
                               new_base,
                               [arr[idx],       arr[idx + 1],
                                arr[idx + 3],   arr[idx + 4]]);
            }
        }
    }
}

impl<F: Field> Field for DiamondSquare<F> {
    fn get_value(&self, pos: V2) -> i32 {
        let level = (self.offsets.len() / 2) as Level;
        let step = scalar(1 << level);
        let base = pos.div_floor(step) * step;
        let init = [
            self.init.get_value(base + step * V2::new(0, 0)),
            self.init.get_value(base + step * V2::new(1, 0)),
            self.init.get_value(base + step * V2::new(0, 1)),
            self.init.get_value(base + step * V2::new(1, 1)),
        ];

        let mut result = 0;
        diamond_square(&|r| r.contains(pos),
                       &mut |p, v| if p == pos { result = v},
                       self.seed,
                       &*self.offsets,
                       base,
                       init);
        result
    }

    fn get_region(&self, bounds: Region2, buf: &mut [i32]) {
        let level = (self.offsets.len() / 2) as Level;
        let step = scalar(1 << level);
        let region_div = bounds.div_round(1 << level);
        for base_div in region_div.points() {
            let base = base_div * step;

            let mut init = [0; 4];
            for corner in Region2::new(scalar(0), scalar(2)).points() {
                let p = base + step * corner;
                let v = self.init.get_value(p);
                if bounds.contains(p) {
                    buf[bounds.index(p)] = v;
                }
                init[(corner.x + 2 * corner.y) as usize] = v;
            }

            diamond_square(&|r| r.overlaps(bounds),
                           &mut |p, v| {
                               if bounds.contains(p) {
                                   buf[bounds.index(p)] = v;
                               }
                           },
                           self.seed,
                           &*self.offsets,
                           base,
                           init);
        }
    }
}


pub struct DiamondSquare<F: Field> {
    seed: u64,
    init: F,
    offsets: Vec<i32>,
}

impl<F: Field> DiamondSquare<F> {
    pub fn new(seed: u64, init: F, offsets: Vec<i32>) -> DiamondSquare<F> {
        DiamondSquare {
            seed: seed,
            init: init,
            offsets: offsets,
        }
    }
}
