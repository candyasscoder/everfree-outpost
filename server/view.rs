use physics::v3::{Vn, V3, V2, scalar, Region};
use physics::{CHUNK_SIZE, TILE_SIZE};

pub const VIEW_SIZE: V2 = V2 { x: 5, y: 6 };
pub const VIEW_ANCHOR: V2 = V2 { x: 2, y: 2 };

pub struct ViewState {
    last: Region<V2>,
}

impl ViewState {
    pub fn new() -> ViewState {
        ViewState {
            last: Region::empty(),
        }
    }

    fn change_region(&mut self, new_region: Region<V2>) -> Option<(Region<V2>, Region<V2>)> {
        let old_region = self.last;

        if new_region == old_region {
            None
        } else {
            self.last = new_region;
            Some((old_region, new_region))
        }
    }

    pub fn update(&mut self, pos: V3) -> Option<(Region<V2>, Region<V2>)> {
        let center = pos.reduce().div_floor(scalar(CHUNK_SIZE * TILE_SIZE));

        let base = center - VIEW_ANCHOR;
        let new_region = Region::new(base, base + VIEW_SIZE);

        self.change_region(new_region)
    }

    pub fn clear(&mut self) -> Option<(Region<V2>, Region<V2>)> {
        self.change_region(Region::empty())
    }

    pub fn region(&self) -> Region<V2> {
        self.last
    }
}
