use rand::Rng;

use libserver_types::*;
use libserver_config::Data;
use libphysics::CHUNK_SIZE;

use GenStructure;
use algo::cellular::CellularGrid;
use StdRng;

pub trait Vault {
    fn pos(&self) -> V2;
    fn size(&self) -> V2;

    fn bounds(&self) -> Region<V2> {
        Region::new(self.pos(), self.pos() + self.size())
    }

    fn connection_points(&self) -> &[V2];

    fn gen_cave_grid(&self,
                     grid: &mut CellularGrid,
                     bounds: Region<V2>) {}

    fn gen_terrain(&self,
                   data: &Data,
                   terrain: &mut [BlockId],
                   bounds: Region<V2>,
                   layer: u8) {}

    fn gen_structures(&self,
                      data: &Data,
                      structures: &mut Vec<GenStructure>,
                      bounds: Region<V2>,
                      layer: u8) {}
}


pub struct FloorMarking {
    pos: V2,
    template_id: TemplateId,
}

impl FloorMarking {
    pub fn new(pos: V2, template_id: TemplateId) -> FloorMarking {
        FloorMarking {
            pos: pos,
            template_id: template_id,
        }
    }
}

impl Vault for FloorMarking {
    fn pos(&self) -> V2 { self.pos }
    fn size(&self) -> V2 { V2::new(1, 1) }

    fn connection_points(&self) -> &[V2] {
        static POINTS: [V2; 1] = [V2 { x: 0, y: 0 }];
        &POINTS
    }

    fn gen_cave_grid(&self,
                     grid: &mut CellularGrid,
                     bounds: Region<V2>) {
        for offset in Region::<V2>::new(scalar(0), scalar(1)).points() {
            if bounds.contains(self.pos + offset) {
                grid.set_fixed(self.pos + offset - bounds.min, false);
            }
        }
    }

    fn gen_structures(&self,
                      _: &Data,
                      structures: &mut Vec<GenStructure>,
                      bounds: Region<V2>,
                      layer: u8) {
        let layer_z = layer as i32 * 2;
        if bounds.contains(self.pos) {
            structures.push(GenStructure::new((self.pos - bounds.min).extend(layer_z),
                                              self.template_id));
        }
    }
}


pub struct Door {
    center: V2,
    corners: (i8, i8),
}

impl Door {
    pub fn new(center: V2, corners: (i8, i8)) -> Door {
        Door {
            center: center,
            corners: corners,
        }
    }
}

impl Vault for Door {
    fn pos(&self) -> V2 { self.center - V2::new(3, 3) }
    fn size(&self) -> V2 { V2::new(7, 7) }

    fn connection_points(&self) -> &[V2] { &[] }

    fn gen_cave_grid(&self,
                     grid: &mut CellularGrid,
                     grid_bounds: Region<V2>) {
        static GRID: [[u8; 8]; 8] = [
            [0, 0, 0, 1, 1, 0, 0, 0],
            [3, 0, 0, 1, 1, 0, 0, 4],
            [3, 3, 1, 1, 1, 1, 4, 4],
            [2, 2, 1, 1, 1, 1, 2, 2],
            [2, 2, 1, 1, 1, 1, 2, 2],
            [5, 5, 1, 1, 1, 1, 6, 6],
            [5, 0, 0, 1, 1, 0, 0, 6],
            [0, 0, 0, 1, 1, 0, 0, 0],
        ];
        let vault_bounds = Region::new(self.pos(), self.pos() + self.size() + scalar(1));
        for pos in vault_bounds.intersect(grid_bounds).points() {
            let offset = pos - self.pos();
            let val = GRID[offset.y as usize][offset.x as usize];
            let setting =
                match val {
                    1 => Some(false),
                    2 => Some(true),
                    3 if self.corners.1 != -1 => Some(true),
                    4 if self.corners.1 !=  1 => Some(true),
                    5 if self.corners.0 != -1 => Some(true),
                    6 if self.corners.0 !=  1 => Some(true),
                    _ => None,
                };
            if let Some(val) = setting {
                grid.set_fixed(pos - grid_bounds.min, val);
            }
        }
    }

    fn gen_terrain(&self,
                   data: &Data,
                   terrain: &mut [BlockId],
                   bounds: Region<V2>,
                   layer: u8) {
        let left = self.center - V2::new(2, 0);
        let right = self.center + V2::new(2, 0);
        let layer_z = layer as i32 * 2;
        let tile_bounds = bounds.extend(0, CHUNK_SIZE);

        if bounds.contains(left) {
            let key = 1*3 + 2*3*3;
            terrain[tile_bounds.index(left.extend(layer_z))] =
                data.block_data.get_id(&format!("cave/{}/z0/dirt", key));
            terrain[tile_bounds.index(left.extend(layer_z + 1))] =
                data.block_data.get_id(&format!("cave/{}/z1", key));
        }

        if bounds.contains(right) {
            let key = 1 + 2*3*3*3;
            terrain[tile_bounds.index(right.extend(layer_z))] =
                data.block_data.get_id(&format!("cave/{}/z0/dirt", key));
            terrain[tile_bounds.index(right.extend(layer_z + 1))] =
                data.block_data.get_id(&format!("cave/{}/z1", key));
        }
    }

    fn gen_structures(&self,
                      data: &Data,
                      structures: &mut Vec<GenStructure>,
                      bounds: Region<V2>,
                      layer: u8) {
        let layer_z = layer as i32 * 2;
        let door_pos = self.center - V2::new(1, 0);
        if bounds.contains(door_pos) {
            let template_id = data.structure_templates.get_id("dungeon/door/key/closed");
            structures.push(GenStructure::new((door_pos - bounds.min).extend(layer_z),
                                              template_id));
        }
    }
}


pub struct Entrance {
    center: V2,
}

impl Entrance {
    pub fn new(center: V2) -> Entrance {
        Entrance {
            center: center,
        }
    }
}

impl Vault for Entrance {
    fn pos(&self) -> V2 { self.center - V2::new(2, 2) }
    fn size(&self) -> V2 { V2::new(5, 5) }

    fn connection_points(&self) -> &[V2] { &[] }

    fn gen_cave_grid(&self,
                     grid: &mut CellularGrid,
                     grid_bounds: Region<V2>) {
        let vault_bounds = Region::new(self.pos(), self.pos() + self.size() + scalar(1));
        for pos in vault_bounds.intersect(grid_bounds).points() {
            grid.set_fixed(pos - grid_bounds.min, false);
        }
    }

    fn gen_structures(&self,
                      data: &Data,
                      structures: &mut Vec<GenStructure>,
                      bounds: Region<V2>,
                      layer: u8) {
        let layer_z = layer as i32 * 2;

        let exit_pos = self.center - V2::new(0, 1);
        if bounds.contains(exit_pos) {
            let template_id = data.structure_templates.get_id("dungeon_exit");
            structures.push(GenStructure::new((exit_pos - bounds.min).extend(layer_z),
                                              template_id));
        }
    }
}


pub enum TreasureKind {
    Chest(u8, &'static str),
    Trophy,
    Fountain,
}

pub struct Treasure {
    center: V2,
    kind: TreasureKind,
}

impl Treasure {
    pub fn new(center: V2, kind: TreasureKind) -> Treasure {
        Treasure {
            center: center,
            kind: kind,
        }
    }
}

impl Vault for Treasure {
    fn pos(&self) -> V2 { self.center - V2::new(1, 1) }
    fn size(&self) -> V2 { V2::new(3, 3) }

    fn connection_points(&self) -> &[V2] { &[] }

    fn gen_cave_grid(&self,
                     grid: &mut CellularGrid,
                     grid_bounds: Region<V2>) {
        let vault_bounds = Region::new(self.pos(), self.pos() + self.size() + scalar(1));
        for pos in vault_bounds.intersect(grid_bounds).points() {
            grid.set_fixed(pos - grid_bounds.min, false);
        }
    }

    fn gen_structures(&self,
                      data: &Data,
                      structures: &mut Vec<GenStructure>,
                      bounds: Region<V2>,
                      layer: u8) {
        let layer_z = layer as i32 * 2;
        if bounds.contains(self.center) {
            let pos = (self.center - bounds.min).extend(layer_z);
            match self.kind {
                TreasureKind::Chest(count, item) => {
                    let template_id = data.structure_templates.get_id("chest");
                    let mut gs = GenStructure::new(pos, template_id);
                    gs.extra.insert("loot".to_owned(), format!("{}:{}", item, count));
                    structures.push(gs);
                },
                TreasureKind::Trophy => {
                    let template_id = data.structure_templates.get_id("trophy");
                    structures.push(GenStructure::new(pos, template_id));
                },
                TreasureKind::Fountain => {
                    let template_id = data.structure_templates.get_id("fountain");
                    structures.push(GenStructure::new(pos, template_id));
                },
            }
        }
    }
}


pub struct Library {
    center: V2,
    size: i32,
    rng: StdRng,
}

impl Library {
    pub fn new(center: V2, size: i32, rng: StdRng) -> Library {
        Library {
            center: center,
            size: size,
            rng: rng,
        }
    }
}

impl Vault for Library {
    fn pos(&self) -> V2 { self.center - scalar(self.size) }
    fn size(&self) -> V2 { scalar(2 * self.size + 1) }

    fn connection_points(&self) -> &[V2] { &[] }

    fn gen_cave_grid(&self,
                     grid: &mut CellularGrid,
                     grid_bounds: Region<V2>) {
        let vault_bounds = Region::new(self.pos(), self.pos() + self.size() + scalar(1));
        for pos in vault_bounds.intersect(grid_bounds).points() {
            grid.set_fixed(pos - grid_bounds.min, false);
        }
    }

    fn gen_structures(&self,
                      data: &Data,
                      structures: &mut Vec<GenStructure>,
                      bounds: Region<V2>,
                      layer: u8) {
        // TODO: kind of a hack
        let mut rng = self.rng.clone();
        let layer_z = layer as i32 * 2;
        let vault_bounds = Region::new(self.pos(), self.pos() + self.size());
        for pos in vault_bounds.intersect(bounds).points() {
            if (pos.y - self.center.y) % 2 != 0 {
                let choice = rng.gen_range(0, 15);
                let amount =
                    if choice < 5 { 0 }
                    else if choice < 8 { 1 }
                    else if choice < 10 { 2 }
                    else { continue; };

                let template_id = data.structure_templates.get_id(
                    &format!("bookshelf/{}", amount));
                structures.push(GenStructure::new((pos - bounds.min).extend(layer_z),
                                                  template_id));
            }
        }
    }
}
