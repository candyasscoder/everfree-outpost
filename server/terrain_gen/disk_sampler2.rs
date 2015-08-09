use std::iter;
use rand::Rng;

use types::*;


pub struct DiskSampler {
    /// List of generated points.
    points: Vec<V2>,

    /// List of preset points, provided by the user.
    init_points: Vec<V2>,

    /// The grid.  Cell size is set such that each cell can contain at most one point.  Each cell
    /// contains the offset of that point within the cell, or NO_POINT.
    grid: Box<[(u16, u16)]>,

    /// Size of the entire area.
    size: V2,

    /// Size of a single cell.  (All cells are square.)
    cell_size: i32,

    min_spacing: i32,
    max_spacing: i32,
}

const NO_POINT: (u16, u16) = (0xffff, 0xffff);

impl DiskSampler {
    pub fn new(size: V2, min_spacing: i32, max_spacing: i32) -> DiskSampler {
        // If each cell is `min_spacing / sqrt(2)` wide and tall, then even two points on opposite
        // corners of the cell would be less than `min_spacing` distance apart.
        let cell_size = min_spacing * 100 / 142;
        let grid_bounds = Region::new(scalar(0), size).div_round(cell_size);
        let grid_size = grid_bounds.size();
        let len = (grid_size.x * grid_size.y) as usize;
        let grid = iter::repeat(NO_POINT).take(len).collect::<Vec<_>>().into_boxed_slice();

        DiskSampler {
            points: Vec::new(),
            init_points: Vec::new(),
            grid: grid,
            size: size,
            cell_size: cell_size,
            min_spacing: min_spacing,
            max_spacing: max_spacing,
        }
    }

    fn bounds(&self) -> Region<V2> {
        Region::new(scalar(0), self.size)
    }

    fn grid_bounds(&self) -> Region<V2> {
        self.bounds().div_round(self.cell_size)
    }

    /// Take a point at random from the queue.  The queue consists of every point in `self.points`
    /// beyond index `*idx`.
    fn choose_one<R: Rng>(&mut self, rng: &mut R, idx: &mut usize) -> Option<V2> {
        let max = self.points.len();
        if *idx >= max {
            return None;
        }

        let choice = rng.gen_range(*idx, max);
        let result = self.points[choice];
        self.points.swap(*idx, choice);
        *idx += 1;
        Some(result)
    }

    fn check_spacing(&self, pos: V2) -> bool {
        if !self.bounds().contains(pos) {
            return false;
        }

        let min_dist2 = self.min_spacing * self.min_spacing;
        let min_bounds = Region::new(pos - scalar(self.min_spacing),
                                     pos + scalar(self.min_spacing + 1));
        let grid_min_bounds = min_bounds.div_round(self.cell_size);
        let grid_bounds = self.grid_bounds();
        for cell in grid_min_bounds.intersect(grid_bounds).points() {
            let grid_idx = grid_bounds.index(cell);
            if self.grid[grid_idx] == NO_POINT {
                continue;
            }

            let (dx, dy) = self.grid[grid_idx];
            let base = cell * scalar(self.cell_size);
            let other_pos = base + V2::new(dx as i32, dy as i32);

            let dist2 = (pos - other_pos).mag2();
            if dist2 < min_dist2 {
                return false;
            }
        }

        true
    }

    fn try_place(&mut self, pos: V2) {
        if self.check_spacing(pos) {
            let cell = pos.div_floor(scalar(self.cell_size));
            info!("successfully placed at {:?} (cell {:?})", pos, cell);
            let cell_idx = self.grid_bounds().index(cell);
            let offset = pos - cell * scalar(self.cell_size);
            self.grid[cell_idx] = (offset.x as u16, offset.y as u16);
            self.points.push(pos);
        }
    }

    fn place_nearby<R: Rng>(&mut self, rng: &mut R, pos: V2, tries: u32) {
        let step = self.max_spacing;
        let bounds = self.bounds();
        let min2 = self.min_spacing * self.min_spacing;
        let max2 = self.max_spacing * self.max_spacing;
        for _ in 0 .. tries {
            let mut candidate = pos;
            // Try to choose a point the right distance from the target.
            for _ in 0 .. 1000 {
                candidate = pos + V2::new(rng.gen_range(-step, step + 1),
                                          rng.gen_range(-step, step + 1));
                let dist2 = (candidate - pos).mag2();
                if dist2 >= min2 && dist2 <= max2 {
                    break;
                }
            }

            self.try_place(candidate);
        }
    }

    pub fn generate<R: Rng>(&mut self, rng: &mut R, tries: u32) {
        info!("populating grid with {} init points", self.init_points.len());
        for i in 0 .. self.init_points.len() {
            let pos = self.init_points[i];
            self.try_place(pos);
        }

        if self.points.len() == 0 {
            let pos = V2::new(rng.gen_range(0, self.size.x),
                              rng.gen_range(0, self.size.y));
            info!("adding random extra point at {:?}", pos);
            self.try_place(pos);
        }

        let mut idx = 0;
        while let Some(pos) = self.choose_one(rng, &mut idx) {
            info!("placing near {:?}...", pos);
            self.place_nearby(rng, pos, tries);
        }
        info!("done! --------------------------");
    }

    pub fn points(&self) -> &[V2] {
        &self.points
    }

    pub fn add_init_point(&mut self, pos: V2) {
        self.init_points.push(pos);
    }
}
