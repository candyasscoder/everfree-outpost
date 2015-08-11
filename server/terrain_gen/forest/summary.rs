use std::fs::File;
use std::io::{self, Read, Write};
use std::iter;
use std::mem;
use linked_hash_map::LinkedHashMap;

use physics::CHUNK_SIZE;
use types::*;

use storage::Storage;
use terrain_gen::cache::Summary;
use util::bytes::*;
use util::Convert;
use util::ReadExact;
use util::{transmute_slice, transmute_slice_mut};


pub struct ChunkSummary {
    /// The value at each diamond-square vertex in the chunk.
    pub ds_levels: [u8; ((CHUNK_SIZE + 1) * (CHUNK_SIZE + 1)) as usize],

    /// For each face, a map of what caves are present on that face.  In each horizontal slice of
    /// the chunk, each connected component is assigned a distinct number.  Only connections within
    /// the same chunk are considered, so two caves connected only through a neighboring chunk will
    /// be assigned distinct cave numbers.  A cave number of 0 indicates that the cell is not part
    /// of a cave (it is either outdoors or solid).
    pub cave_nums: [[[u8; CHUNK_SIZE as usize + 1]; 4]; CHUNK_SIZE as usize / 2],

    /// Map of internal connectivity between caves on different levels.  Cave number 0 indicates
    /// outside, for caves that open directly onto the surface.
    ///
    /// It should always be the case that every cave in the chunk is accessible from the surface or
    /// through some adjacent chunk.
    pub cave_connectivity: Vec<(u8, u8)>,

    /// Offsets of all trees/rocks in the chunk.
    pub tree_offsets: Vec<V2>,
}

impl Summary for ChunkSummary {
    fn alloc() -> Box<ChunkSummary> {
        Box::new(ChunkSummary {
            ds_levels: unsafe { mem::zeroed() },
            cave_nums: unsafe { mem::zeroed() },
            cave_connectivity: Vec::new(),
            tree_offsets: Vec::new(),
        })
    }

    fn write_to(&self, mut f: File) -> io::Result<()> {
        try!(f.write_all(&self.ds_levels));

        for layer in &self.cave_nums {
            for edge in layer {
                try!(f.write_all(edge));
            }
        }

        // Length of cave_connectivity should never exceed 255 * 256 < u16::MAX
        try!(f.write_bytes(self.cave_connectivity.len().to_u16().unwrap()));
        try!(f.write_all(unsafe { transmute_slice(&self.cave_connectivity) }));

        try!(f.write_bytes(self.tree_offsets.len().to_u16().unwrap()));
        try!(f.write_all(unsafe { transmute_slice(&self.tree_offsets) }));

        Ok(())
    }

    fn read_from(mut f: File) -> io::Result<Box<ChunkSummary>> {
        let mut summary = ChunkSummary::alloc();

        try!(f.read_exact(&mut summary.ds_levels));

        for layer in &mut summary.cave_nums {
            for edge in layer {
                try!(f.read_exact(edge));
            }
        }

        let cave_connectivity_len = try!(f.read_bytes::<u16>()) as usize;
        summary.cave_connectivity = iter::repeat((0, 0)).take(cave_connectivity_len).collect();
        try!(f.read_exact(unsafe { transmute_slice_mut(&mut summary.cave_connectivity) }));

        let tree_offsets_len = try!(f.read_bytes::<u16>()) as usize;
        summary.tree_offsets = iter::repeat(scalar(0)).take(tree_offsets_len).collect();
        try!(f.read_exact(unsafe { transmute_slice_mut(&mut summary.tree_offsets) }));

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
