use std::cmp;
use std::collections::lru_cache::LruCache;
use std::rand::{Rng, XorShiftRng, SeedableRng};


// TODO: Move this to a shared utility library

/// Representation of a 2D region.
struct Region {
    x0: i32,
    y0: i32,
    x1: i32,
    y1: i32,
}

impl Region {
    fn new(x0: i32, y0: i32, x1: i32, y1: i32) -> Region {
        Region {
            x0: x0,
            y0: y0,
            x1: cmp::max(x0, x1),
            y1: cmp::max(y0, y1),
        }
    }

    fn around(x: i32, y: i32, radius: i32) -> Region {
        Region::new(x - radius, y - radius, x + radius, y + radius)
    }

    fn contains(&self, x: i32, y: i32) -> bool {
        self.x0 <= x && x < self.x1 &&
        self.y0 <= y && y < self.y1
    }

    fn contains_inclusive(&self, x: i32, y: i32) -> bool {
        self.x0 <= x && x <= self.x1 &&
        self.y0 <= y && y <= self.y1
    }

    fn intersect(&self, other: &Region) -> Region {
        let x0 = cmp::max(self.x0, other.x0);
        let y0 = cmp::max(self.y0, other.y0);
        let x1 = cmp::min(self.x1, other.x1);
        let y1 = cmp::min(self.y1, other.y1);
        Region::new(x0, y0, x1, y1)
    }

    fn points(&self) -> Points {
        Points {
            x: self.x0,
            y: if self.x0 < self.x1 { self.y0 } else { self.y1 },
            min_x: self.x0,
            max_x: self.x1,
            max_y: self.y1,
        }
    }

    fn index(&self, x: i32, y: i32) -> uint {
        let dx = (x - self.x0) as uint;
        let dy = (y - self.y0) as uint;
        let w = (self.x1 - self.x0) as uint;

        dy * w + dx
    }
}

struct Points {
    x: i32,
    y: i32,
    min_x: i32,
    max_x: i32,
    max_y: i32,
}

impl Iterator<(i32, i32)> for Points {
    fn next(&mut self) -> Option<(i32, i32)> {
        if self.y >= self.max_y {
            return None;
        }

        let result = Some((self.x, self.y));

        self.x += 1;
        if self.x >= self.max_x {
            self.x = self.min_x;
            self.y += 1;
        }
        result
    }
}



/// Core loop of Poisson disk sampling.  Calls `place` to place each point and calls `choose` to
/// choose `tries` new points after each successful placement.  The points in `init` are placed
/// first and used to initialize the queue.
fn disk_sample<T, Place, Choose, R>(mut place: Place,
                                    mut choose: Choose,
                                    rng: &mut R,
                                    init: &[T],
                                    tries: uint)
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
        let x0 = queue.swap_remove(idx).unwrap();

        for _ in range(0, tries) {
            let x = choose(x0);
            if place(x) {
                queue.push(x);
            }
        }
    }
}


fn min_space(_x: i32, _y: i32) -> i32 {
    //2 + y / 8
    3
}


/// Place trees using Poisson disk sampling.  Uses a `grid` (whose dimensions are defined by
/// `grid_area`) to track trees placed so far.  Returns two lists of points `(inside, outside)`,
/// containing the trees that were placed within the `bounds` and those that were attempted to be
/// placed outside the `bounds`.  The grid is prepopulated by placing all the trees listed in
/// `prepopulate`.
fn place_trees<R: Rng>(grid: &mut [u8],
                       grid_area: Region,
                       bounds: Region,
                       prepopulate: &[&[(i32, i32)]],
                       rng: &mut R) -> (Vec<(i32, i32)>, Vec<(i32, i32)>) {
    let mut inside = Vec::new();
    let mut outside = Vec::new();

    {
        let mut place_unchecked = |&mut: grid: &mut [u8], (tx, ty): (i32, i32)| {
            let space = min_space(tx, ty);
            let space_sq = space * space;

            for (x, y) in Region::around(tx, ty, space).intersect(&grid_area).points() {
                // Measure from the center of the cell.  Otherwise the circle will be uneven,
                // since tx,ty refers to a grid intersection, not a cell.
                let dx = (x - tx) * 2 + 1;
                let dy = (y - ty) * 2 + 1;
                let dist_sq = dx * dx + dy * dy;
                if dist_sq > 4 * space_sq {
                    continue;
                }

                let idx = grid_area.index(x, y);
                // It is possible for grid[idx] to be 2 (tree) when a tree is placed with large
                // min_space immediately outside the excluded zone of a tree with smaller
                // min_space.
                if grid[idx] == 0 {
                    grid[idx] = 1;
                }
            }

            for (x,y) in Region::around(tx, ty, 1).intersect(&grid_area).points() {
                grid[grid_area.index(x, y)] = 2;
            }

            if bounds.contains_inclusive(tx, ty) {
                inside.push((tx, ty));
                true
            } else {
                outside.push((tx, ty));
                false
            }
        };

        // Prepopulate grid with data from adjacent regions.
        for points in prepopulate.iter() {
            for &point in points.iter() {
                place_unchecked(grid, point);
            }
        }

        let place = |&mut: (tx, ty): (i32, i32)| {
            if !grid_area.contains_inclusive(tx, ty) {
                return false;
            }

            for (x,y) in Region::around(tx, ty, 1).intersect(&grid_area).points() {
                if grid[grid_area.index(x, y)] != 0 {
                    return false;
                }
            }

            place_unchecked(grid, (tx, ty))
        };

        let mut choose_rng = rng.gen::<XorShiftRng>();
        let choose = |&mut: (tx, ty): (i32, i32)| {
            let min_space = min_space(tx, ty);
            let max_space = min_space * 2;

            let mut dx = choose_rng.gen_range(-max_space, max_space + 1);
            let mut dy = choose_rng.gen_range(-max_space, max_space + 1);
            while dx * dx + dy * dy <= min_space * min_space ||
                  dx * dx + dy * dy >= max_space * max_space {
                dx = choose_rng.gen_range(-max_space, max_space + 1);
                dy = choose_rng.gen_range(-max_space, max_space + 1);
            }
            (tx + dx, ty + dy)
        };


        let mut init_rng = rng.gen::<XorShiftRng>();
        let mut init = Vec::with_capacity(5);
        for _ in range(0u, 5) {
            let point = (init_rng.gen_range(bounds.x0, bounds.x1),
                         init_rng.gen_range(bounds.y0, bounds.y1));
            init.push(point);
        }

        let mut sample_rng = rng.gen::<XorShiftRng>();
        disk_sample(place, choose, &mut sample_rng, init.as_slice(), 30);
    }

    (inside, outside)
}

struct GenData {
    inside: Vec<(i32, i32)>,
    outside: Vec<(i32, i32)>,
}

#[deriving(PartialEq, Eq, Hash, Show)]
enum Section {
    Corner,
    Top,
    Left,
    Center,
}

const GEN_CHUNK_BITS: uint = 7;
const GEN_CHUNK_SIZE: i32 = 128;
const FRINGE_SIZE: i32 = 32;
const CORNER_SIZE: i32 = 32;
const EDGE_SIZE: i32 = GEN_CHUNK_SIZE - CORNER_SIZE;

const GRID_SIZE: i32 = GEN_CHUNK_SIZE + 2 * FRINGE_SIZE;

/// Process the dependencies of a section by running `process` on each one.  The dependencies of
/// any section are guaranteed to have higher priority than the section itself, where priority is
/// ordered by `Corner > Top/Left > Center`.
fn process_deps<F>(mut process: F, x: i32, y: i32, section: Section)
        where F: FnMut(i32, i32, Section) {
    match section {
        Corner => {},
        Top => {
            process(x, y, Corner);
            process(x + 1, y, Corner);
        },
        Left => {
            process(x, y, Corner);
            process(x, y + 1, Corner);
        },
        Center => {
            process(x, y, Top);
            process(x, y, Left);
            process(x, y + 1, Top);
            process(x + 1, y, Left);
        },
    }
}

/// Generate trees within a section and return the `inside` and `outside` points.  Uses `fetch` to
/// fetch data for the dependencies of the chunk.
fn generate_chunk_data<'a, F, R>(mut fetch: F,
                                 x: i32, y: i32,
                                 section: Section,
                                 rng: &mut R) -> GenData
        where F: FnMut(i32, i32, Section) -> &'a GenData,
              R: Rng {
    let mut prepopulate = Vec::new();
    {
        let include = |&mut: x: i32, y: i32, section: Section| {
            let data = fetch(x, y, section);
            prepopulate.push(data.inside.as_slice());
            prepopulate.push(data.outside.as_slice());
        };
        process_deps(include, x, y, section);
    }

    let (off_x, off_y, width, height) = match section {
        Corner => (0, 0, CORNER_SIZE, CORNER_SIZE),
        Top => (CORNER_SIZE, 0, EDGE_SIZE, CORNER_SIZE),
        Left => (0, CORNER_SIZE, CORNER_SIZE, EDGE_SIZE),
        Center => (CORNER_SIZE, CORNER_SIZE, EDGE_SIZE, EDGE_SIZE),
    };

    let mut grid = [0, ..(GRID_SIZE * GRID_SIZE) as uint];
    let bounds = Region::new(x * GEN_CHUNK_SIZE + off_x,
                             y * GEN_CHUNK_SIZE + off_y,
                             x * GEN_CHUNK_SIZE + off_x + width,
                             y * GEN_CHUNK_SIZE + off_y + height);
    let grid_area = Region::new(bounds.x0 - FRINGE_SIZE,
                                bounds.y0 - FRINGE_SIZE,
                                bounds.x1 + FRINGE_SIZE,
                                bounds.y1 + FRINGE_SIZE);

    let (inside, outside) = place_trees(grid.as_mut_slice(),
                                        grid_area,
                                        bounds,
                                        prepopulate.as_slice(),
                                        rng);
    GenData {
        inside: inside,
        outside: outside,
    }
}

/// Generate trees for a section and insert the result into an `LruCache`.  This function calls
/// itself recursively to produce `GenData` for the section's dependencies, if it is not already
/// available.
///
/// The provided `LruCache` must have capacity of at least 9.  (Otherwise, the chunk's dependencies
/// could be evicted before the chunk itself can be generated.)
fn generate(data: &mut LruCache<(i32, i32, Section), GenData>,
            x: i32, y: i32, section: Section,
            seed: u64) {
    {
        let process = |&mut: x: i32, y: i32, section: Section| {
            if data.get(&(x, y, section)).is_none() {
                generate(data, x, y, section, seed);
            }
        };
        process_deps(process, x, y, section);
    }

    let chunk_data = {
        let fetch = |&mut: x: i32, y: i32, section: Section| {
            data.get(&(x, y, section)).unwrap()
        };
        let seed = [seed as u32, (seed >> 32) as u32,
                    x as u32 << 2 | section as u32,
                    y as u32];
        let mut rng = SeedableRng::from_seed(seed);
        generate_chunk_data(fetch, x, y, section, &mut rng)
    };
    data.insert((x, y, section), chunk_data);
}


fn section_for_point(px: i32, py: i32) -> (i32, i32, Section) {
    let sx = px >> GEN_CHUNK_BITS;
    let sy = py >> GEN_CHUNK_BITS;
    let offset_x = px & ((1 << GEN_CHUNK_BITS) - 1);
    let offset_y = py & ((1 << GEN_CHUNK_BITS) - 1);

    let section = match (offset_x < CORNER_SIZE, offset_y < CORNER_SIZE) {
        (true, true) => Corner,
        (true, false) => Left,
        (false, true) => Top,
        (false, false) => Center,
    };
    (sx, sy, section)
}

fn section_for_chunk(cx: i32, cy: i32) -> (i32, i32, Section) {
    use physics::CHUNK_SIZE;
    section_for_point(cx * CHUNK_SIZE, cy * CHUNK_SIZE)
}

fn sections_around_chunk(cx: i32, cy: i32, buf: &mut [(i32, i32, Section), ..4]) -> &[(i32, i32, Section)] {
    let mut pos = 0;

    {
        let mut push = |&mut: item: (i32, i32, Section)| {
            if buf.slice_to(pos).contains(&item) {
                return;
            }
            buf[pos] = item;
            pos += 1;
        };

        push(section_for_chunk(cx,     cy));
        push(section_for_chunk(cx - 1, cy));
        push(section_for_chunk(cx + 1, cy));
        push(section_for_chunk(cx,     cy - 1));
        push(section_for_chunk(cx,     cy + 1));
        push(section_for_chunk(cx - 1, cy - 1));
        push(section_for_chunk(cx + 1, cy - 1));
        push(section_for_chunk(cx - 1, cy + 1));
        push(section_for_chunk(cx + 1, cy + 1));
    }

    buf.slice_to(pos)
}


const LRU_SIZE: uint = 1024;

pub struct TerrainGenerator {
    seed: u64,
    gen_data: LruCache<(i32, i32, Section), GenData>,
}

impl TerrainGenerator {
    pub fn new(seed: u64) -> TerrainGenerator {
        TerrainGenerator {
            seed: seed,
            gen_data: LruCache::new(LRU_SIZE),
        }
    }

    pub fn generate_chunk(&mut self, cx: i32, cy: i32) -> ::state::Chunk {
        use physics::{CHUNK_BITS, CHUNK_SIZE};
        const Z_STEP: uint = 1 << (2 * CHUNK_BITS);

        let mut chunk = [0, ..1 << (3 * CHUNK_BITS)];
        for i in range(0, 1 << (2 * CHUNK_BITS)) {
            chunk[i] = 1;
        }

        let mut sections_buf = [(0, 0, Corner), ..4];
        let sections = sections_around_chunk(cx, cy, &mut sections_buf);

        let bounds = Region::new(cx * CHUNK_SIZE,
                                 cy * CHUNK_SIZE,
                                 (cx + 1) * CHUNK_SIZE,
                                 (cy + 1) * CHUNK_SIZE);

        for &(sx, sy, section) in sections.iter() {
            generate(&mut self.gen_data, sx, sy, section, self.seed);
            let data = self.gen_data.get(&(sx, sy, section)).unwrap();
            for &(tx, ty) in data.inside.iter() {
                if bounds.contains(tx - 2, ty) {
                    chunk[bounds.index(tx - 2, ty) + 0 * Z_STEP] = 9;
                }

                if bounds.contains(tx - 2, ty - 1) {
                    chunk[bounds.index(tx - 2, ty - 1) + 0 * Z_STEP] = 8;
                }

                if bounds.contains(tx - 1, ty) {
                    chunk[bounds.index(tx - 1, ty) + 0 * Z_STEP] = 6;
                    chunk[bounds.index(tx - 1, ty) + 1 * Z_STEP] = 16;
                    chunk[bounds.index(tx - 1, ty) + 2 * Z_STEP] = 12;
                }

                if bounds.contains(tx - 1, ty - 1) {
                    chunk[bounds.index(tx - 1, ty - 1) + 0 * Z_STEP] = 5;
                    chunk[bounds.index(tx - 1, ty - 1) + 1 * Z_STEP] = 5;
                    chunk[bounds.index(tx - 1, ty - 1) + 2 * Z_STEP] = 13;
                }

                if bounds.contains(tx, ty) {
                    chunk[bounds.index(tx, ty) + 0 * Z_STEP] = 7;
                    chunk[bounds.index(tx, ty) + 1 * Z_STEP] = 17;
                    chunk[bounds.index(tx, ty) + 2 * Z_STEP] = 14;
                }

                if bounds.contains(tx, ty - 1) {
                    chunk[bounds.index(tx, ty - 1) + 0 * Z_STEP] = 5;
                    chunk[bounds.index(tx, ty - 1) + 1 * Z_STEP] = 5;
                    chunk[bounds.index(tx, ty - 1) + 2 * Z_STEP] = 15;
                }

                if bounds.contains(tx + 1, ty) {
                    chunk[bounds.index(tx + 1, ty) + 0 * Z_STEP] = 11;
                }

                if bounds.contains(tx + 1, ty - 1) {
                    chunk[bounds.index(tx + 1, ty - 1) + 0 * Z_STEP] = 10;
                }
            }
        }

        chunk
    }
}
