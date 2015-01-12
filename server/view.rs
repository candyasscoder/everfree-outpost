use physics::v3::V3;
use physics::{CHUNK_BITS, TILE_BITS};

pub const VIEW_SIZE_X: i32 = 5;
pub const VIEW_SIZE_Y: i32 = 6;
pub const VIEW_ANCHOR_X: i32 = 2;
pub const VIEW_ANCHOR_Y: i32 = 2;

pub struct ViewState {
    last_center: (i32, i32),
}

#[derive(Show)]
pub struct ViewRegion {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl ViewRegion {
    fn around(center: (i32, i32)) -> ViewRegion {
        let (cx, cy) = center;
        ViewRegion {
            x: cx - VIEW_ANCHOR_X,
            y: cy - VIEW_ANCHOR_Y,
            w: VIEW_SIZE_X,
            h: VIEW_SIZE_Y,
        }
    }

    pub fn contains(&self, x: i32, y: i32) -> bool {
        self.x <= x && x < self.x + self.w &&
        self.y <= y && y < self.y + self.h
    }

    pub fn points(&self) -> Points {
        Points {
            x: self.x,
            y: self.y,
            min_x: self.x,
            max_x: self.x + self.w,
            max_y: self.y + self.h,
        }
    }
}

pub struct Points {
    x: i32,
    y: i32,
    min_x: i32,
    max_x: i32,
    max_y: i32,
}

impl Iterator for Points {
    type Item = (i32, i32);
    fn next(&mut self) -> Option<(i32, i32)> {
        while self.y < self.max_y && self.x >= self.max_x {
            self.x = self.min_x;
            self.y += 1;
        }

        if self.y >= self.max_y {
            None
        } else {
            let result = (self.x, self.y);
            self.x += 1;
            Some(result)
        }
    }
}

impl ViewState {
    pub fn new(pos: V3) -> ViewState {
        let center = (pos.x >> (CHUNK_BITS + TILE_BITS),
                      pos.y >> (CHUNK_BITS + TILE_BITS));

        ViewState { last_center: center }
    }

    pub fn update(&mut self, pos: V3) -> Option<(ViewRegion, ViewRegion)> {
        let center = (pos.x >> (CHUNK_BITS + TILE_BITS),
                      pos.y >> (CHUNK_BITS + TILE_BITS));

        let old_center = self.last_center;
        self.last_center = center;

        if center == old_center {
            None
        } else {
            Some((ViewRegion::around(old_center),
                  ViewRegion::around(center)))
        }
    }

    pub fn region(&self) -> ViewRegion {
        ViewRegion::around(self.last_center)
    }
}
