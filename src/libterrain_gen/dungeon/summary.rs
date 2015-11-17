use std::fs::File;
use std::io::{self, Write};
use std::mem;
use std::ptr;

use libphysics::CHUNK_SIZE;
use libserver_types::*;
use libserver_util::{BitSlice, Convert, ReadExact};
use libserver_util::{transmute_slice, transmute_slice_mut};
use libserver_util::{write_vec, read_vec};
use libserver_util::bytes::*;

use cache::Summary;
use super::vault::{Vault, read_vault};
use super::types::Triangle;


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
    /// Edges in the graph.  A passage will be placed roughly along each edge.
    // TODO: Box<[(V2, V2)]>?
    pub edges: Vec<(V2, V2)>,

    pub neg_edges: Vec<(V2, V2)>,

    pub tris: Vec<Triangle>,

    /// Vaults to be placed in the generated terrain.
    // TODO: wish we could use fewer allocations here...
    pub vaults: Vec<Box<Vault>>,

    pub verts: Vec<V2>,
}

impl Summary for PlaneSummary {
    fn alloc() -> Box<PlaneSummary> {
        Box::new(PlaneSummary {
            edges: Vec::new(),
            neg_edges: Vec::new(),
            tris: Vec::new(),
            vaults: Vec::new(),
            verts: Vec::new(),
        })
    }

    fn write_to(&self, mut f: File) -> io::Result<()> {
        try!(unsafe { write_vec(&mut f, &self.edges) });
        try!(unsafe { write_vec(&mut f, &self.neg_edges) });
        try!(unsafe { write_vec(&mut f, &self.tris) });

        try!(f.write_bytes(self.vaults.len().to_u32().unwrap()));
        for v in &self.vaults {
            v.write_to(&mut f);
        }

        Ok(())
    }

    fn read_from(mut f: File) -> io::Result<Box<PlaneSummary>> {
        let mut summary = PlaneSummary::alloc();

        summary.edges = try!(unsafe { read_vec(&mut f) });
        summary.neg_edges = try!(unsafe { read_vec(&mut f) });
        summary.tris = try!(unsafe { read_vec(&mut f) });

        let vaults_count = try!(f.read_bytes::<u32>()) as usize;
        summary.vaults = Vec::with_capacity(vaults_count);
        for _ in 0 .. vaults_count {
            summary.vaults.push(try!(read_vault(&mut f)));
        }

        Ok(summary)
    }
}
