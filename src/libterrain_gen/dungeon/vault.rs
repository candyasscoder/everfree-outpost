use std::fs::File;
use std::io::{self, Write};
use rand::{self, Rng};

use libserver_types::*;
use libserver_config::Data;
use libphysics::CHUNK_SIZE;
use libserver_util::{Convert, ReadExact};
use libserver_util::{transmute_slice, transmute_slice_mut};
use libserver_util::{write_array, read_array};
use libserver_util::{write_vec, read_vec};
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


#[derive(Clone, Copy, Debug)]
pub enum StructureKind {
    Trophy,
    Fountain,
}
unsafe impl Bytes for StructureKind {}

impl StructureKind {
    fn name(&self) -> &'static str {
        use self::StructureKind::*;
        match *self {
            Trophy => "trophy",
            Fountain => "fountain",
        }
    }
}

pub struct Structure {
    pos: V2,
    kind: StructureKind,
}

impl Structure {
    pub fn new(pos: V2, kind: StructureKind) -> Structure {
        Structure {
            pos: pos,
            kind: kind,
        }
    }
}

impl Vault for Structure {
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
                      data: &Data,
                      structures: &mut Vec<GenStructure>,
                      bounds: Region<V2>,
                      layer: u8) {
        let layer_z = layer as i32 * 2;
        if bounds.contains(self.pos) {
            let template_id = data.structure_templates.get_id(self.kind.name());
            structures.push(GenStructure::new((self.pos - bounds.min).extend(layer_z),
                                              template_id));
        }
    }

    fn write_to(&self, f: &mut File) -> io::Result<()> {
        try!(f.write_bytes(1_u8));
        try!(f.write_bytes(self.pos));
        try!(f.write_bytes(self.kind));
        Ok(())
    }
}

impl VaultRead for Structure {
    fn read_from(f: &mut File) -> io::Result<Box<Structure>> {
        let pos = try!(f.read_bytes());
        let kind = try!(f.read_bytes());
        Ok(Box::new(Structure {
            pos: pos,
            kind: kind,
        }))
    }
}


#[derive(Clone, Copy)]
pub enum DoorKind {
    Key,
    GemPuzzle,
}
unsafe impl Bytes for DoorKind {}

impl DoorKind {
    fn template_name(&self) -> &'static str {
        use self::DoorKind::*;
        match *self {
            Key => "dungeon/door/key/closed",
            GemPuzzle => "dungeon/door/puzzle/closed",
        }
    }
}

pub struct Door {
    center: V2,
    kind: DoorKind,
    area: u32,
}

impl Door {
    pub fn new(center: V2,
               kind: DoorKind,
               area: u32) -> Door {
        Door {
            center: center,
            kind: kind,
            area: area,
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
            let template_id = data.structure_templates.get_id(self.kind.template_name());
            let pos = (door_pos - bounds.min).extend(layer_z);
            let mut gs = GenStructure::new(pos, template_id);
            match self.kind {
                DoorKind::GemPuzzle => {
                    gs.extra.insert("gem_puzzle_door".to_owned(),
                                    format!("{}_{}", layer, self.area));
                },
                _ => {},
            }
            structures.push(gs);
        }
    }

    fn write_to(&self, f: &mut File) -> io::Result<()> {
        try!(f.write_bytes(2_u8));
        try!(f.write_bytes(self.center));
        try!(f.write_bytes(self.area));
        try!(f.write_bytes(self.kind));
        Ok(())
    }
}

impl VaultRead for Door {
    fn read_from(f: &mut File) -> io::Result<Box<Door>> {
        let center = try!(f.read_bytes());
        let area = try!(f.read_bytes());
        let kind = try!(f.read_bytes());
        Ok(Box::new(Door {
            center: center,
            area: area,
            kind: kind,
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


#[derive(Clone, Copy, Debug)]
pub enum ChestItem {
    Hat,
    Key,
    Book,
    Gem(GemColor),
}
unsafe impl Bytes for ChestItem {}

impl ChestItem {
    fn data_name(self) -> &'static str {
        match self {
            ChestItem::Hat => "hat",
            ChestItem::Key => "key",
            ChestItem::Book => "book",
            ChestItem::Gem(c) => c.item_name(),
        }
    }
}

pub struct Chest {
    center: V2,
    contents: Vec<(ChestItem, u8)>,
}

impl Chest {
    pub fn new(center: V2, contents: Vec<(ChestItem, u8)>) -> Chest {
        Chest {
            center: center,
            contents: contents,
        }
    }
}

impl Vault for Chest {
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
            let template_id = data.structure_templates.get_id("chest");
            let mut gs = GenStructure::new(pos, template_id);

            let mut loot_str = String::new();
            for &(item, count) in &self.contents {
                loot_str.push_str(&format!("{}:{},", item.data_name(), count));
            }
            gs.extra.insert("loot".to_owned(), loot_str);
            structures.push(gs);
        }
    }

    fn write_to(&self, f: &mut File) -> io::Result<()> {
        try!(f.write_bytes(4_u8));
        try!(f.write_bytes(self.center));
        try!(unsafe { write_vec(f, &self.contents) });
        Ok(())
    }
}

impl VaultRead for Chest {
    fn read_from(f: &mut File) -> io::Result<Box<Chest>> {
        let center = try!(f.read_bytes());
        let contents = try!(unsafe { read_vec(f) });
        Ok(Box::new(Chest {
            center: center,
            contents: contents,
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


#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GemColor {
    Red,
    Orange,
    Yellow,
    Green,
    Blue,
    Purple,
}

impl GemColor {
    fn template_name(&self) -> &'static str {
        use self::GemColor::*;
        match *self {
            Red =>      "dungeon/gem_slot/fixed/red",
            Orange =>   "dungeon/gem_slot/fixed/orange",
            Yellow =>   "dungeon/gem_slot/fixed/yellow",
            Green =>    "dungeon/gem_slot/fixed/green",
            Blue =>     "dungeon/gem_slot/fixed/blue",
            Purple =>   "dungeon/gem_slot/fixed/purple",
        }
    }

    fn item_name(&self) -> &'static str {
        use self::GemColor::*;
        match *self {
            Red =>      "gem/red",
            Orange =>   "gem/orange",
            Yellow =>   "gem/yellow",
            Green =>    "gem/green",
            Blue =>     "gem/blue",
            Purple =>   "gem/purple",
        }
    }

    fn name(&self) -> &'static str {
        use self::GemColor::*;
        match *self {
            Red =>      "red",
            Orange =>   "orange",
            Yellow =>   "yellow",
            Green =>    "green",
            Blue =>     "blue",
            Purple =>   "purple",
        }
    }

    pub fn index(self) -> u8 {
        use self::GemColor::*;
        match self {
            Red =>      0,
            Orange =>   1,
            Yellow =>   2,
            Green =>    3,
            Blue =>     4,
            Purple =>   5,
        }
    }

    pub fn from_index(index: u8) -> GemColor {
        use self::GemColor::*;
        match index {
            0 => Red,
            1 => Orange,
            2 => Yellow,
            3 => Green,
            4 => Blue,
            5 => Purple,
            _ => panic!("invalid GemColor index: {}", index),
        }
    }

    pub fn blend(self, other: GemColor) -> GemColor {
        use self::GemColor::*;
        match (self, other) {
            (Red, Yellow) | (Yellow, Red) => Orange,
            (Yellow, Blue) | (Blue, Yellow) => Green,
            (Red, Blue) | (Blue, Red) => Purple,
            _ => panic!("can't mix secondary colors: {:?}, {:?}", self, other),
        }
    }

    pub fn cycle(self, n: i8) -> GemColor {
        let i = self.index() as i16 + n as i16;
        if i >= 0 {
            GemColor::from_index((i % 6) as u8)
        } else {
            let m = -i % 6;
            if m == 0 {
                GemColor::from_index(0)
            } else {
                GemColor::from_index((6 - m) as u8)
            }
        }
    }
}

pub struct GemPuzzle {
    center: V2,
    colors: Box<[Option<GemColor>]>,
    area: u32,
}

impl GemPuzzle {
    pub fn new(center: V2,
               area: u32,
               colors: Box<[Option<GemColor>]>) -> GemPuzzle {
        GemPuzzle {
            center: center,
            colors: colors,
            area: area,
        }
    }

    fn inner_size(&self) -> V2 {
        V2::new(self.colors.len() as i32, 1)
    }
}

impl Vault for GemPuzzle {
    fn pos(&self) -> V2 { self.center - self.inner_size() / scalar(2) - scalar(1) }
    fn size(&self) -> V2 { self.inner_size() + V2::new(2, 3) }

    fn connection_points(&self) -> &[V2] {
        static POINTS: [V2; 1] = [V2 { x: 0, y: 0 }];
        &POINTS
    }

    fn gen_cave_grid(&self,
                     grid: &mut CellularGrid,
                     bounds: Region<V2>) {
        let vault_bounds = Region::new(self.pos(), self.pos() + self.size() + scalar(1));
        for pos in vault_bounds.intersect(bounds).points() {
            grid.set_fixed(pos - bounds.min, false);
        }
    }

    fn gen_structures(&self,
                      data: &Data,
                      structures: &mut Vec<GenStructure>,
                      bounds: Region<V2>,
                      layer: u8) {
        let layer_z = layer as i32 * 2;
        let inner_base = self.center - self.inner_size() / scalar(2);
        for (i, &c) in self.colors.iter().enumerate() {
            let (name, template_name) =
                if let Some(c) = c {
                    (c.name(), c.template_name())
                } else {
                    ("empty", "dungeon/gem_slot/normal/empty")
                };
            let template_id = data.structure_templates.get_id(template_name);
            let pos = inner_base + V2::new(i as i32, 0);
            if !bounds.contains(pos) {
                continue;
            }
            let mut gs = GenStructure::new((pos - bounds.min).extend(layer_z), template_id);
            gs.extra.insert("gem_puzzle_slot".to_owned(),
                            format!("{}_{},{},{}", layer, self.area, i, name));
            structures.push(gs);
        }
    }

    fn write_to(&self, f: &mut File) -> io::Result<()> {
        try!(f.write_bytes(6_u8));
        try!(f.write_bytes(self.center));
        try!(f.write_bytes(self.area));
        try!(unsafe { write_array(f, &self.colors) });

        Ok(())
    }
}

impl VaultRead for GemPuzzle {
    fn read_from(f: &mut File) -> io::Result<Box<GemPuzzle>> {
        let center = try!(f.read_bytes());
        let area = try!(f.read_bytes());
        let colors = try!(unsafe { read_array(f) });
        Ok(Box::new(GemPuzzle {
            center: center,
            colors: colors,
            area: area,
        }))
    }
}



pub fn read_vault(f: &mut File) -> io::Result<Box<Vault>> {
    match try!(f.read_bytes::<u8>()) {
        1 => Ok(try!(Structure::read_from(f))),
        2 => Ok(try!(Door::read_from(f))),
        3 => Ok(try!(Entrance::read_from(f))),
        4 => Ok(try!(Chest::read_from(f))),
        5 => Ok(try!(Library::read_from(f))),
        6 => Ok(try!(GemPuzzle::read_from(f))),
        _ => panic!("bad vault tag in summary"),
    }
}
