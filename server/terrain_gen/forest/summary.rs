use std::fs::File;
use std::io::{self, Read, Write};
use std::iter;
use std::mem;
use std::ptr;
use linked_hash_map::LinkedHashMap;

use physics::CHUNK_SIZE;
use types::*;

use storage::Storage;
use terrain_gen::cache::Summary;
use util::{BitSlice, Convert, ReadExact};
use util::{transmute_slice, transmute_slice_mut};
use util::bytes::*;


pub struct ChunkSummary {
    /// The value at each diamond-square vertex in the chunk.
    pub heightmap: [u8; ((CHUNK_SIZE + 1) * (CHUNK_SIZE + 1)) as usize],

    /// A bit for each vertex, 0 for cave interior and 1 for walls (or "not inside a cave").  This
    /// field is private because callers should use the methods returning `BitSlice` rather than
    /// accessing it directly.
    cave_walls: [[u8;
            (((CHUNK_SIZE + 1) * (CHUNK_SIZE + 1) + 7) / 8) as usize];
            (CHUNK_SIZE / 2) as usize],

    /// Offsets of all trees/rocks in the chunk.
    pub tree_offsets: Vec<V2>,

    /// Offsets of treasure (cave structures) on each layer of the chunk.
    pub treasure_offsets: [Vec<V2>; CHUNK_SIZE as usize / 2],
}

impl ChunkSummary {
    pub fn cave_wall_layer(&self, layer: u8) -> &BitSlice {
        BitSlice::from_bytes(&self.cave_walls[layer as usize])
    }

    pub fn cave_wall_layer_mut(&mut self, layer: u8) -> &mut BitSlice {
        BitSlice::from_bytes_mut(&mut self.cave_walls[layer as usize])
    }
}

impl Summary for ChunkSummary {
    fn alloc() -> Box<ChunkSummary> {
        let mut treasure_offsets = unsafe {
            let mut arr = mem::zeroed();
            for p in &mut arr {
                ptr::write(p as *mut _, Vec::<V2>::new());
            }
            arr
        };

        Box::new(ChunkSummary {
            heightmap: unsafe { mem::zeroed() },
            cave_walls: unsafe { mem::zeroed() },
            tree_offsets: Vec::new(),
            treasure_offsets: treasure_offsets,
        })
    }

    fn write_to(&self, mut f: File) -> io::Result<()> {
        try!(f.write_all(&self.heightmap));

        for layer in &self.cave_walls {
            try!(f.write_all(layer));
        }

        try!(f.write_bytes(self.tree_offsets.len().to_u16().unwrap()));
        try!(f.write_all(unsafe { transmute_slice(&self.tree_offsets) }));

        for layer in &self.treasure_offsets {
            try!(f.write_bytes(layer.len().to_u16().unwrap()));
            try!(f.write_all(unsafe { transmute_slice(&layer) }));
        }

        Ok(())
    }

    fn read_from(mut f: File) -> io::Result<Box<ChunkSummary>> {
        let mut summary = ChunkSummary::alloc();

        try!(f.read_exact(&mut summary.heightmap));

        for layer in &mut summary.cave_walls {
            try!(f.read_exact(layer));
        }

        let tree_offsets_len = try!(f.read_bytes::<u16>()) as usize;
        summary.tree_offsets = iter::repeat(scalar(0)).take(tree_offsets_len).collect();
        try!(f.read_exact(unsafe { transmute_slice_mut(&mut summary.tree_offsets) }));

        for layer in &mut summary.treasure_offsets {
            let layer_len = try!(f.read_bytes::<u16>()) as usize;
            *layer = iter::repeat(scalar(0)).take(layer_len).collect();
            try!(f.read_exact(unsafe { transmute_slice_mut(layer) }));
        }

        Ok(summary)
    }
}


pub const SUPERCHUNK_BITS: usize = 5;
pub const SUPERCHUNK_SIZE: i32 = 1 << SUPERCHUNK_BITS;

pub struct SuperchunkSummary {
    pub ds_levels: [u8; ((SUPERCHUNK_SIZE + 1) * (SUPERCHUNK_SIZE + 1)) as usize],
}

impl Summary for SuperchunkSummary {
    fn alloc() -> Box<SuperchunkSummary> {
        Box::new(SuperchunkSummary {
            ds_levels: unsafe { mem::zeroed() },
        })
    }

    fn write_to(&self, mut f: File) -> io::Result<()> {
        try!(f.write_all(&self.ds_levels));

        Ok(())
    }

    fn read_from(mut f: File) -> io::Result<Box<SuperchunkSummary>> {
        let mut summary = SuperchunkSummary::alloc();

        try!(f.read_exact(&mut summary.ds_levels));

        Ok(summary)
    }
}
