use std::fs::File;
use std::io::{self, Write};
use rand::{self, Rng};

use libserver_types::*;
use libserver_config::Data;
use libphysics::CHUNK_SIZE;
use libserver_util::{Convert, ReadExact};
use libserver_util::{transmute_slice, transmute_slice_mut};
use libserver_util::bytes::*;

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

    fn write_to(&self, f: &mut File) -> io::Result<()>;
}

pub trait VaultRead: Vault {
    fn read_from(f: &mut File) -> io::Result<Box<Self>>;
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

    fn write_to(&self, f: &mut File) -> io::Result<()> {
        try!(f.write_bytes(1_u8));
        try!(f.write_bytes(self.pos));
        try!(f.write_bytes(self.template_id));
        Ok(())
    }
}

impl VaultRead for FloorMarking {
    fn read_from(f: &mut File) -> io::Result<Box<FloorMarking>> {
        let pos = try!(f.read_bytes());
        // FIXME: this will break badly if the data files change
        let template_id = try!(f.read_bytes());
        Ok(Box::new(FloorMarking {
            pos: pos,
            template_id: template_id,
        }))
    }
}


pub struct Door {
    center: V2,
}

impl Door {
    pub fn new(center: V2) -> Door {
        Door {
            center: center,
        }
    }
}

impl Vault for Door {
    fn pos(&self) -> V2 { self.center - V2::new(2, 2) }
    fn size(&self) -> V2 { V2::new(5, 5) }

    fn connection_points(&self) -> &[V2] { &[] }

    fn gen_cave_grid(&self,
                     grid: &mut CellularGrid,
                     grid_bounds: Region<V2>) {
        static GRID: [[u8; 6]; 6] = [
            [0, 0, 1, 1, 0, 0],
            [0, 1, 1, 1, 1, 0],
            [2, 1, 1, 1, 1, 2],
            [1, 1, 1, 1, 1, 1],
            [0, 1, 1, 1, 1, 0],
            [0, 0, 1, 1, 0, 0],
        ];
        let vault_bounds = Region::new(self.pos(), self.pos() + self.size() + scalar(1));
        for pos in vault_bounds.intersect(grid_bounds).points() {
            let offset = pos - self.pos();
            let val = GRID[offset.y as usize][offset.x as usize];
            let setting =
                match val {
                    1 => Some(false),
                    2 => Some(true),
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
            let key = 1*3 + 2*3*3 + 2*3*3*3;
            terrain[tile_bounds.index(left.extend(layer_z))] =
                data.block_data.get_id(&format!("cave/{}/z0/dirt", key));
            terrain[tile_bounds.index(left.extend(layer_z + 1))] =
                data.block_data.get_id(&format!("cave/{}/z1", key));
        }

        if bounds.contains(right) {
            let key = 1 + 2*3*3 + 2*3*3*3;
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

    fn write_to(&self, f: &mut File) -> io::Result<()> {
        try!(f.write_bytes(2_u8));
        try!(f.write_bytes(self.center));
        Ok(())
    }
}

impl VaultRead for Door {
    fn read_from(f: &mut File) -> io::Result<Box<Door>> {
        let center = try!(f.read_bytes());
        let corners = try!(f.read_bytes());
        Ok(Box::new(Door {
            center: center,
        }))
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

    fn write_to(&self, f: &mut File) -> io::Result<()> {
        try!(f.write_bytes(3_u8));
        try!(f.write_bytes(self.center));
        Ok(())
    }
}

impl VaultRead for Entrance {
    fn read_from(f: &mut File) -> io::Result<Box<Entrance>> {
        let center = try!(f.read_bytes());
        Ok(Box::new(Entrance {
            center: center,
        }))
    }
}


#[derive(Clone, Copy)]
pub enum ChestItem {
    Hat,
    Key,
    Book,
}

impl ChestItem {
    fn data_name(self) -> &'static str {
        match self {
            ChestItem::Hat => "hat",
            ChestItem::Key => "key",
            ChestItem::Book => "book",
        }
    }
}

#[derive(Clone, Copy)]
pub enum TreasureKind {
    Chest(u8, ChestItem),
    Trophy,
    Fountain,
}

unsafe impl Bytes for TreasureKind {}

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
                    gs.extra.insert("loot".to_owned(),
                                    format!("{}:{}", item.data_name(), count));
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

    fn write_to(&self, f: &mut File) -> io::Result<()> {
        try!(f.write_bytes(4_u8));
        try!(f.write_bytes(self.center));
        try!(f.write_bytes(self.kind));
        Ok(())
    }
}

impl VaultRead for Treasure {
    fn read_from(f: &mut File) -> io::Result<Box<Treasure>> {
        let center = try!(f.read_bytes());
        let kind = try!(f.read_bytes());
        Ok(Box::new(Treasure {
            center: center,
            kind: kind,
        }))
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

    fn write_to(&self, f: &mut File) -> io::Result<()> {
        try!(f.write_bytes(5_u8));
        try!(f.write_bytes(self.center));
        try!(f.write_bytes(self.size));
        Ok(())
    }
}

impl VaultRead for Library {
    fn read_from(f: &mut File) -> io::Result<Box<Library>> {
        let center = try!(f.read_bytes());
        let size = try!(f.read_bytes());
        let rng = rand::random();
        Ok(Box::new(Library {
            center: center,
            size: size,
            rng: rng,
        }))
    }
}


pub struct GemPuzzle {
    center: V2,
    size: V2,
}

impl GemPuzzle {
    pub fn new(center: V2, size: V2) -> GemPuzzle {
        GemPuzzle {
            center: center,
            size: size,
        }
    }
}

impl Vault for GemPuzzle {
    fn pos(&self) -> V2 { self.center - self.size / scalar(2) - scalar(1) }
    fn size(&self) -> V2 { self.size + scalar(2) }

    fn connection_points(&self) -> &[V2] {
        static POINTS: [V2; 1] = [V2 { x: 0, y: 0 }];
        &POINTS
    }

    fn gen_cave_grid(&self,
                     grid: &mut CellularGrid,
                     bounds: Region<V2>) {
        for pos in self.bounds().intersect(bounds).points() {
            grid.set_fixed(pos - bounds.min, false);
        }
    }

    fn gen_structures(&self,
                      _: &Data,
                      structures: &mut Vec<GenStructure>,
                      bounds: Region<V2>,
                      layer: u8) {
        let layer_z = layer as i32 * 2;
        for pos in self.bounds().expand(scalar(-1)).intersect(bounds).points() {
            structures.push(GenStructure::new((pos - bounds.min).extend(layer_z),
                                              0));
        }
    }

    fn write_to(&self, f: &mut File) -> io::Result<()> {
        try!(f.write_bytes(6_u8));
        try!(f.write_bytes(self.center));
        try!(f.write_bytes(self.size));
        Ok(())
    }
}

impl VaultRead for GemPuzzle {
    fn read_from(f: &mut File) -> io::Result<Box<GemPuzzle>> {
        let center = try!(f.read_bytes());
        let size = try!(f.read_bytes());
        Ok(Box::new(GemPuzzle {
            center: center,
            size: size,
        }))
    }
}



pub fn read_vault(f: &mut File) -> io::Result<Box<Vault>> {
    match try!(f.read_bytes::<u8>()) {
        1 => Ok(try!(FloorMarking::read_from(f))),
        2 => Ok(try!(Door::read_from(f))),
        3 => Ok(try!(Entrance::read_from(f))),
        4 => Ok(try!(Treasure::read_from(f))),
        5 => Ok(try!(Library::read_from(f))),
        6 => Ok(try!(GemPuzzle::read_from(f))),
        _ => panic!("bad vault tag in summary"),
    }
}
