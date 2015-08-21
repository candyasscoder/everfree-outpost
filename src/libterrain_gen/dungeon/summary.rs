use std::fs::File;
use std::io::{self, Write};
use std::mem;
use std::ptr;

use libphysics::CHUNK_SIZE;
use libserver_types::*;
use libserver_util::{BitSlice, Convert, ReadExact};
use libserver_util::{transmute_slice, transmute_slice_mut};
use libserver_util::bytes::*;

use cache::Summary;


// TODO: copied from forest::summary; move to somewhere common
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


pub struct ChunkSummary {
    /// A bit for each vertex, 0 for cave interior and 1 for walls (or "not inside a cave").  This
    /// field is private because callers should use the methods returning `BitSlice` rather than
    /// accessing it directly.
    cave_walls: [u8; (((CHUNK_SIZE + 1) * (CHUNK_SIZE + 1) + 7) / 8) as usize],
}

impl ChunkSummary {
    pub fn cave_walls(&self) -> &BitSlice {
        BitSlice::from_bytes(&self.cave_walls)
    }

    pub fn cave_walls_mut(&mut self) -> &mut BitSlice {
        BitSlice::from_bytes_mut(&mut self.cave_walls)
    }
}

impl Summary for ChunkSummary {
    fn alloc() -> Box<ChunkSummary> {
        Box::new(ChunkSummary {
            cave_walls: unsafe { mem::zeroed() },
        })
    }

    fn write_to(&self, mut f: File) -> io::Result<()> {
        try!(f.write_all(&self.cave_walls));

        Ok(())
    }

    fn read_from(mut f: File) -> io::Result<Box<ChunkSummary>> {
        let mut summary = ChunkSummary::alloc();

        try!(f.read_exact(&mut summary.cave_walls));

        Ok(summary)
    }
}


pub struct PlaneSummary {
    /// Location of every vertex in the graph that controls the high-level structure of the plane.
    pub vertices: Vec<V2>,

    /// Edges in the graph.  Each endpoint is the index of an element of `vertices`.
    pub edges: Vec<(u16, u16)>,
}

impl Summary for PlaneSummary {
    fn alloc() -> Box<PlaneSummary> {
        Box::new(PlaneSummary {
            vertices: Vec::new(),
            edges: Vec::new(),
        })
    }

    fn write_to(&self, mut f: File) -> io::Result<()> {
        try!(unsafe { write_vec(&mut f, &self.vertices) });
        try!(unsafe { write_vec(&mut f, &self.edges) });

        Ok(())
    }

    fn read_from(mut f: File) -> io::Result<Box<PlaneSummary>> {
        let mut summary = PlaneSummary::alloc();

        summary.vertices = try!(unsafe { read_vec(&mut f) });
        summary.edges = try!(unsafe { read_vec(&mut f) });

        Ok(summary)
    }
}