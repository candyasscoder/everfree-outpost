use physics::CHUNK_BITS;
use LOCAL_BITS;


/// Tile numbers used to display a particular block.
#[derive(Clone, Copy)]
pub struct BlockData {
    // 0
    pub front: u16,
    pub back: u16,
    pub top: u16,
    pub bottom: u16,

    // 8
    pub light_color: (u8, u8, u8),
    pub _pad1: u8,
    pub light_radius: u16,
    pub _pad2: u16,

    // 16
}

impl BlockData {
    pub fn tile(&self, side: usize) -> u16 {
        match side {
            0 => self.front,
            1 => self.back,
            2 => self.top,
            3 => self.bottom,
            _ => panic!("invalid side number"),
        }
    }
}


/// A chunk of terrain.  Each element is a block ID.
pub type BlockChunk = [u16; 1 << (3 * CHUNK_BITS)];
/// BlockChunk for every chunk in the local region.
pub type LocalChunks = [BlockChunk; 1 << (2 * LOCAL_BITS)];


pub struct StructureTemplate {
    // 0
    pub size: (u8, u8, u8),
    pub sheet: u8,
    pub display_size: (u16, u16),
    pub display_offset: (u16, u16),
    pub layer: u8,

    // 13
    pub anim_sheet: u8,
    pub anim_length: i8,
    pub anim_rate: u8,
    pub anim_offset: (u16, u16),
    pub anim_pos: (u16, u16),
    pub anim_size: (u8, u8),

    // 26
    pub light_pos: (u8, u8, u8),
    pub light_color: (u8, u8, u8),
    pub light_radius: u16,

    // 34
}
