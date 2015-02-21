use std::cell::RefCell;
use std::iter::repeat;
use std::rand::{Rng, XorShiftRng, SeedableRng};

use collect::lru_cache::LruCache;

use data::BlockData;
use physics::v3::{Vn, V3, V2, Region, Region2, scalar};
use types::BlockChunk;

use self::Section::*;

// TODO: Use V2 instead of (i32, i32) for chunk coordinates



/// Core loop of Poisson disk sampling.  Calls `place` to place each point and calls `choose` to
/// choose `tries` new points after each successful placement.  The points in `init` are placed
/// first and used to initialize the queue.
fn disk_sample<T, Place, Choose, R>(mut place: Place,
                                    mut choose: Choose,
                                    rng: &mut R,
                                    init: &[T],
                                    tries: usize)
        where T: Copy + ::std::fmt::Show,
              Place: FnMut(T) -> bool,
              Choose: FnMut(T) -> T,
              R: Rng {

    let mut queue = Vec::new();

    for &x in init.iter(){ 
        if place(x) {
            queue.push(x);
        }
    }

    while queue.len() > 0 {
        let idx = rng.gen_range(0, queue.len());
        let x0 = queue.swap_remove(idx);

        for _ in range(0, tries) {
            let x = choose(x0);
            if place(x) {
                queue.push(x);
            }
        }
    }
}

/// Higher-level disk sampling routine.  Fill `bounds` with points, where each point is
/// at least distance `get_spacing(point)` from all other points (including points from `initial`,
/// which are not included in the output).  All values returned by `get_spacing` should lie within
/// `min_spacing`..`max_spacing`.
fn disk_sample2<R, GS>(rng: &mut R,
                       bounds: Region2,
                       min_spacing: i32,
                       max_spacing: i32,
                       get_spacing: &GS,
                       initial: &[V2]) -> Vec<V2>
        where R: Rng,
              GS: Fn(V2) -> i32 {
    let mut rng = rng;
    let get_spacing = |&: pos| { (*get_spacing)(pos) };

    // Choose cell size such that a circle of radius `min_spacing` centered anywhere in the cell
    // must always cover the entire cell.
    let cell_size = min_spacing * 100 / 142;

    // All points of interest to the current set of placements lie within `max_spacing` of the
    // current `bounds`.
    let outer_bounds = bounds.expand(scalar(max_spacing));
    let grid_bounds = (outer_bounds - outer_bounds.min).div_round(cell_size);
    let mut grid: Vec<Option<V2>> = repeat(None).take(grid_bounds.volume() as usize).collect();
    let mut points = Vec::new();

    for &pos in initial.iter() {
        if !outer_bounds.contains(pos) {
            continue;
        }
        let grid_pos = (pos - outer_bounds.min) / scalar(cell_size);
        grid[grid_bounds.index(grid_pos)] = Some(pos);
    }

    {
        let place = |&mut: pos: V2| {
            if !bounds.contains(pos) {
                return false;
            }

            let spacing = get_spacing(pos);

            // Make sure there are no points within a `spacing` radius of `pos`.

            let around = Region::around(pos, spacing);
            let grid_around = (around - outer_bounds.min).div_round(cell_size);

            // Inspect each grid cell, and for each that contains a point, check if the point is
            // too close to `pos`.
            for grid_pos in grid_around.points() {

                match grid[grid_bounds.index(grid_pos)] {
                    None => {},
                    Some(neighbor_pos) => {
                        let delta = neighbor_pos - pos;
                        let dist2 = delta.dot(delta);
                        if dist2 < spacing * spacing {
                            return false;
                        }
                    },
                }
            }

            points.push(pos);
            let grid_pos = (pos - outer_bounds.min) / scalar(cell_size);
            grid[grid_bounds.index(grid_pos)] = Some(pos);
            true
        };

        let mut choose_rng = rng.gen::<XorShiftRng>();
        let choose = |&mut: pos: V2| {
            let min_space = get_spacing(pos);
            let max_space = min_space * 2;

            let mut dx = choose_rng.gen_range(-max_space, max_space + 1);
            let mut dy = choose_rng.gen_range(-max_space, max_space + 1);
            while dx * dx + dy * dy <= min_space * min_space ||
                  dx * dx + dy * dy >= max_space * max_space {
                dx = choose_rng.gen_range(-max_space, max_space + 1);
                dy = choose_rng.gen_range(-max_space, max_space + 1);
            }
            pos + V2::new(dx, dy)
        };

        let mut init_rng = rng.gen::<XorShiftRng>();
        let mut init = Vec::with_capacity(5);
        for _ in range(0u32, 5) {
            let x = init_rng.gen_range(bounds.min.x, bounds.max.x);
            let y = init_rng.gen_range(bounds.min.y, bounds.max.y);
            init.push(V2::new(x, y));
        }

        let mut sample_rng = rng.gen::<XorShiftRng>();
        disk_sample(place, choose, &mut sample_rng, init.as_slice(), 30);
    }

    points
}


trait PointSource {
    fn generate_points(&self, bounds: Region2) -> Vec<V2>;
}


struct IsoDiskSampler<GS: Fn(V2) -> i32> {
    base_seed: u64,
    min_spacing: i32,
    max_spacing: i32,
    get_spacing: GS,
    chunk_size: i32,
    cache: RefCell<LruCache<(i32, i32, Section), Vec<V2>>>,
}

impl<GS: Fn(V2) -> i32> IsoDiskSampler<GS> {
    fn new(seed: u64,
           min_spacing: u16,
           max_spacing: u16,
           chunk_size: u16,
           get_spacing: GS) -> IsoDiskSampler<GS> {
        IsoDiskSampler {
            base_seed: seed,
            min_spacing: min_spacing as i32,
            max_spacing: max_spacing as i32,
            get_spacing: get_spacing,
            chunk_size: chunk_size as i32,
            cache: RefCell::new(LruCache::new(LRU_SIZE)),
        }
    }

    fn get_chunk<'c>(&self,
                     cache: &'c mut LruCache<(i32, i32, Section), Vec<V2>>,
                     x: i32,
                     y: i32,
                     section: Section) -> &'c [V2] {
        let key = (x, y, section);
        // Can't use Entry API here because we need to take a borrow on `self` to call
        // `generate_chunk`.
        if cache.get(&key).is_none() {
            self.generate_chunk(cache, x, y, section);
        }
        cache.get(&key).unwrap().as_slice()
    }

    fn generate_chunk(&self,
                      cache: &mut LruCache<(i32, i32, Section), Vec<V2>>,
                      x: i32,
                      y: i32,
                      section: Section) {
        let mut initial = Vec::new();
        match section {
            Corner => { },
            Top => {
                initial.push_all(self.get_chunk(cache,  x,     y,     Corner));
                initial.push_all(self.get_chunk(cache,  x + 1, y,     Corner));
            },
            Left => {
                initial.push_all(self.get_chunk(cache,  x,     y,     Corner));
                initial.push_all(self.get_chunk(cache,  x - 1, y,     Top));
                initial.push_all(self.get_chunk(cache,  x,     y,     Top));

                initial.push_all(self.get_chunk(cache,  x,     y + 1, Corner));
                initial.push_all(self.get_chunk(cache,  x - 1, y + 1, Top));
                initial.push_all(self.get_chunk(cache,  x,     y + 1, Top));
            },
            Center => {
                initial.push_all(self.get_chunk(cache,  x,     y,     Corner));
                initial.push_all(self.get_chunk(cache,  x + 1, y,     Corner));
                initial.push_all(self.get_chunk(cache,  x + 1, y + 1, Corner));
                initial.push_all(self.get_chunk(cache,  x,     y + 1, Corner));

                initial.push_all(self.get_chunk(cache,  x,     y,     Top));
                initial.push_all(self.get_chunk(cache,  x,     y,     Left));
                initial.push_all(self.get_chunk(cache,  x,     y + 1, Top));
                initial.push_all(self.get_chunk(cache,  x + 1, y,     Left));
            },
        }

        let seed = [self.base_seed as u32 + 12345,
                    (self.base_seed >> 32) as u32 ^ section as u32,
                    x as u32,
                    y as u32];
        let mut rng: XorShiftRng = SeedableRng::from_seed(seed);

        let bounds = section_bounds(x, y, section, self.chunk_size);
        let data = disk_sample2(&mut rng,
                                bounds,
                                self.min_spacing,
                                self.max_spacing,
                                &self.get_spacing,
                                initial.as_slice());
        cache.insert((x, y, section), data);
    }
}

impl<GS: Fn(V2) -> i32> PointSource for IsoDiskSampler<GS> {
    fn generate_points(&self, bounds: Region2) -> Vec<V2> {
        let mut cache = self.cache.borrow_mut();

        let mut points = Vec::new();
        for pos in bounds.div_round_signed(self.chunk_size * CHUNK_MULT).points() {
            let V2 { x, y } = pos;
            for &section in [Corner, Top, Left, Center].iter() {
                let cur_bounds = section_bounds(x, y, section, self.chunk_size);
                if cur_bounds.intersect(bounds).volume() == 0 {
                    continue;
                }

                points.extend(self.get_chunk(&mut *cache, x, y, section).iter()
                                  .map(|&pos| pos)
                                  .filter(|&pos| bounds.contains(pos)));
            }
        }
        points
    }
}


// We want to place objects in a chunk using Poisson disk sampling, but we need to ensure we get
// the same results no matter what order we generate chunks in.  We manage this by dividing the
// world into a particular type of grid:
//
//      +-+-+
//      |*|*|
//      +-+-+
//      |*|*|
//      +-+-+
//
// Sections marked `+` are called corners, `-` and `|` are edges, and `*` marks centers.  We fill
// in corners first, then edges, and finally centers.  Corners are placed far enough apart that
// they can be generated completely independently.  Horizontal edges are generated next, so they
// depend only on the two adjacent corners.  Vertical edges depend on the two corners and the four
// horizontal edges connected to those corners.  Finally, centers depend on the four adjacent edges
// and four adjacent corners.

#[derive(Copy, PartialEq, Eq, Hash, Show)]
enum Section {
    Corner,
    Top,
    Left,
    Center,
}

const LRU_SIZE: usize = 1024;

const CORNER_MULT: i32 = 1;
const EDGE_MULT: i32 = 3;
const CHUNK_MULT: i32 = 4;

fn section_bounds(x: i32, y: i32, section: Section, chunk_size: i32) -> Region2 {
    let (off_x, off_y, width, height) = match section {
        Corner => (0, 0, CORNER_MULT, CORNER_MULT),
        Top => (CORNER_MULT, 0, EDGE_MULT, CORNER_MULT),
        Left => (0, CORNER_MULT, CORNER_MULT, EDGE_MULT),
        Center => (CORNER_MULT, CORNER_MULT, EDGE_MULT, EDGE_MULT),
    };

    let chunk_min = V2::new(x, y) * scalar(CHUNK_MULT) + V2::new(off_x, off_y);
    let min = chunk_min * scalar(chunk_size);
    let size = V2::new(width, height) * scalar(chunk_size);
    Region::new(min, min + size)
}


pub struct TerrainGenerator {
    seed: u64,
    sampler: Box<PointSource+'static>,
}

impl TerrainGenerator {
    pub fn new(seed: u64) -> TerrainGenerator {
        TerrainGenerator {
            seed: seed,
            sampler: Box::new(IsoDiskSampler::new(seed, 4, 4, 32, |&: _| 4)) as Box<PointSource>,
        }
    }

    pub fn generate_chunk(&self,
                          block_data: &BlockData,
                          cx: i32, cy: i32) -> (BlockChunk, Vec<V3>) {
        use physics::{CHUNK_BITS, CHUNK_SIZE};
        const Z_STEP: usize = 1 << (2 * CHUNK_BITS);

        let seed = [self.seed as u32 + 12345,
                    (self.seed >> 32) as u32,
                    cx as u32,
                    cy as u32];
        let mut rng: XorShiftRng = SeedableRng::from_seed(seed);
        let mut chunk = [0; 1 << (3 * CHUNK_BITS)];
        let ids = [block_data.get_id("grass/center/v0"),
                   block_data.get_id("grass/center/v1"),
                   block_data.get_id("grass/center/v2"),
                   block_data.get_id("grass/center/v3")];
        for i in range(0, 1 << (2 * CHUNK_BITS)) {
            chunk[i] = *rng.choose(ids.as_slice()).unwrap();
        }

        let bounds = Region::new(V2::new(cx, cy), V2::new(cx + 1, cy + 1)) * scalar(CHUNK_SIZE);

        let points = self.sampler.generate_points(bounds.expand(V2::new(2, 2)));
        let points = points.into_iter().map(|pos| (pos - V2::new(2, 1)).extend(0)).collect();

        (chunk, points)
    }
}
