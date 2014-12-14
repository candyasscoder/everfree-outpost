use std::cmp;
use std::collections::lru_cache::LruCache;
use std::rand::{Rng, XorShiftRng, SeedableRng};

use block_data::BlockData;
use physics::v3::{V3, Region};

use self::Section::*;



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
    5
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

            let center = V3::new(tx, ty, 0);
            for pos in Region::around(center, space).intersect(&grid_area).points() {
                let V3 { x, y, z: _ } = pos;

                // Measure from the center of the cell.  Otherwise the circle will be uneven,
                // since tx,ty refers to a grid intersection, not a cell.
                let dx = (x - tx) * 2 + 1;
                let dy = (y - ty) * 2 + 1;
                let dist_sq = dx * dx + dy * dy;
                if dist_sq > 4 * space_sq {
                    continue;
                }

                let idx = grid_area.index(&pos);
                // It is possible for grid[idx] to be 2 (tree) when a tree is placed with large
                // min_space immediately outside the excluded zone of a tree with smaller
                // min_space.
                if grid[idx] == 0 {
                    grid[idx] = 1;
                }
            }

            let tile_center = V3::new(tx, ty, 0);
            for tile_pos in Region::around(tile_center, 1).intersect(&grid_area).points() {
                grid[grid_area.index(&tile_pos)] = 2;
            }

            if bounds.contains_inclusive(&tile_center) {
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
            let tile_pos = V3::new(tx, ty, 0);
            if !grid_area.contains_inclusive(&tile_pos) {
                return false;
            }

            for pos in Region::around(tile_pos, 1).intersect(&grid_area).points() {
                if grid[grid_area.index(&pos)] != 0 {
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
            let point = (init_rng.gen_range(bounds.min.x, bounds.max.x),
                         init_rng.gen_range(bounds.min.y, bounds.max.y));
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
    let bounds = Region::new(V3::new(x * GEN_CHUNK_SIZE + off_x,
                                     y * GEN_CHUNK_SIZE + off_y,
                                     0),
                             V3::new(x * GEN_CHUNK_SIZE + off_x + width,
                                     y * GEN_CHUNK_SIZE + off_y + height,
                                     1));
    let grid_area = Region::new(V3::new(bounds.min.x - FRINGE_SIZE,
                                        bounds.min.y - FRINGE_SIZE,
                                        0),
                                V3::new(bounds.max.x + FRINGE_SIZE,
                                        bounds.max.y + FRINGE_SIZE,
                                        1));

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

    pub fn generate_chunk(&mut self,
                          block_data: &BlockData,
                          cx: i32, cy: i32) -> ::state::Chunk {
        use physics::{CHUNK_BITS, CHUNK_SIZE};
        const Z_STEP: uint = 1 << (2 * CHUNK_BITS);

        let seed = [self.seed as u32 + 12345,
                    (self.seed >> 32) as u32,
                    cx as u32 << 2,
                    cy as u32];
        let mut rng: XorShiftRng = SeedableRng::from_seed(seed);
        let mut chunk = [0, ..1 << (3 * CHUNK_BITS)];
        let ids = [block_data.get_id("grass/center/v0"),
                   block_data.get_id("grass/center/v1"),
                   block_data.get_id("grass/center/v2"),
                   block_data.get_id("grass/center/v3")];
        for i in range(0, 1 << (2 * CHUNK_BITS)) {
            chunk[i] = *rng.choose(ids.as_slice()).unwrap();
        }

        let mut sections_buf = [(0, 0, Corner), ..4];
        let sections = sections_around_chunk(cx, cy, &mut sections_buf);

        let bounds = Region::new(V3::new(cx * CHUNK_SIZE,
                                         cy * CHUNK_SIZE,
                                         0),
                                 V3::new((cx + 1) * CHUNK_SIZE,
                                         (cy + 1) * CHUNK_SIZE,
                                         1));

        {
            let mut set = |&mut: x: i32, y: i32, z: i32, name: &str| {
                let base_pos = V3::new(x, y, 0);
                if bounds.contains(&base_pos) {
                    chunk[bounds.index(&base_pos) + z as uint * Z_STEP] = block_data.get_id(name);
                }
            };

            for &(sx, sy, section) in sections.iter() {
                generate(&mut self.gen_data, sx, sy, section, self.seed);
                let data = self.gen_data.get(&(sx, sy, section)).unwrap();
                for &(tx, ty) in data.inside.iter() {
                    set(tx - 2, ty,     0, "tree/base/left/y1");
                    set(tx - 2, ty - 1, 0, "tree/base/left/y0");
                    set(tx + 1, ty,     0, "tree/base/right/y1");
                    set(tx + 1, ty - 1, 0, "tree/base/right/y0");

                    set(tx - 1, ty,     0, "tree/base/center/x0");
                    set(tx,     ty,     0, "tree/base/center/x1");
                    set(tx - 1, ty,     1, "tree/trunk/00");
                    set(tx,     ty,     1, "tree/trunk/10");
                    set(tx - 1, ty,     2, "tree/top/cutoff/00");
                    set(tx,     ty,     2, "tree/top/cutoff/10");
                    set(tx - 1, ty - 1, 2, "tree/top/cutoff/01");
                    set(tx,     ty - 1, 2, "tree/top/cutoff/11");

                    set(tx - 1, ty - 1, 0, "tree/back");
                    set(tx,     ty - 1, 0, "tree/back");
                    set(tx - 1, ty - 1, 1, "tree/back");
                    set(tx,     ty - 1, 1, "tree/back");
                }
            }

            if cx == 0 && cy == 0 {
                for x in range(5, 9) {
                    for y in range(2, 4) {
                        set(x, y, 2, "floor/wood");
                    }
                }
                set(6, 4, 1, "stair/n");
                set(7, 4, 1, "stair/n");
                set(6, 5, 0, "stair/n");
                set(7, 5, 0, "stair/n");
            }
        }

        chunk
    }
}
