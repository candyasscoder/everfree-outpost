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

    pub heightmap_constraints: Vec<(V2, (u8, u8))>,

    /// A bit for each vertex, 0 for cave interior and 1 for walls (or "not inside a cave").  This
    /// field is private because callers should use the methods returning `BitSlice` rather than
    /// accessing it directly.
    cave_walls: [[u8;
            (((CHUNK_SIZE + 1) * (CHUNK_SIZE + 1) + 7) / 8) as usize];
            (CHUNK_SIZE / 2) as usize],

    pub cave_wall_constraints: [Vec<(V2, bool)>; CHUNK_SIZE as usize / 2],

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

fn vec_per_layer<T>() -> [Vec<T>; CHUNK_SIZE as usize / 2] {
    unsafe {
        let mut arr = mem::zeroed();
        for p in &mut arr {
            ptr::write(p as *mut _, Vec::<T>::new());
        }
        arr
    }
}

unsafe fn write_vec<T>(f: &mut File, v: &Vec<T>) -> io::Result<()> {
    try!(f.write_bytes(v.len().to_u32().unwrap()));
    try!(f.write_all(transmute_slice(v)));
    Ok(())
}

unsafe fn read_vec<T>(f: &mut File) -> io::Result<Vec<T>> {
    let len = try!(f.read_bytes::<u32>()) as usize;
    let mut v = Vec::with_capacity(len);
    v.set_len(len);
    try!(f.read_exact(transmute_slice_mut(&mut v)));
    Ok(v)
}

impl Summary for ChunkSummary {
    fn alloc() -> Box<ChunkSummary> {
        Box::new(ChunkSummary {
            heightmap: unsafe { mem::zeroed() },
            heightmap_constraints: Vec::new(),
            cave_walls: unsafe { mem::zeroed() },
            cave_wall_constraints: vec_per_layer(),
            tree_offsets: Vec::new(),
            treasure_offsets: vec_per_layer(),
        })
    }

    fn write_to(&self, mut f: File) -> io::Result<()> {
        try!(f.write_all(&self.heightmap));
        try!(unsafe { write_vec(&mut f, &self.heightmap_constraints) });

        for i in 0 .. self.cave_walls.len() {
            try!(f.write_all(&self.cave_walls[i]));
            try!(unsafe { write_vec(&mut f, &self.cave_wall_constraints[i]) });
        }

        try!(unsafe { write_vec(&mut f, &self.tree_offsets) });

        for layer in &self.treasure_offsets {
            try!(unsafe { write_vec(&mut f, layer) });
        }

        Ok(())
    }

    fn read_from(mut f: File) -> io::Result<Box<ChunkSummary>> {
        let mut summary = ChunkSummary::alloc();

        try!(f.read_exact(&mut summary.heightmap));
        summary.heightmap_constraints = try!(unsafe { read_vec(&mut f) });

        for i in 0 .. summary.cave_walls.len() {
            try!(f.read_exact(&mut summary.cave_walls[i]));
            summary.cave_wall_constraints[i] = try!(unsafe { read_vec(&mut f) });
        }

        summary.tree_offsets = try!(unsafe { read_vec(&mut f) });

        for layer in &mut summary.treasure_offsets {
            *layer = try!(unsafe { read_vec(&mut f) });
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
