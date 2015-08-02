use std::fs::File;
use std::io::{self, Read, Write};
use std::iter;
use std::mem;
use linked_hash_map::LinkedHashMap;

use physics::CHUNK_SIZE;
use types::*;

use storage::Storage;
use util::bytes::*;
use util::Convert;
use util::ReadExact;
use util::{transmute_slice, transmute_slice_mut};


pub trait Summary {
    fn alloc() -> Box<Self>;
    fn write_to(&self, f: File) -> io::Result<()>;
    fn read_from(f: File) -> io::Result<Box<Self>>;
}


pub type EachEdge<T> = [T; 4];
pub type EachCell1<T> = [T; CHUNK_SIZE as usize];
pub type EachCell2<T> = [T; (CHUNK_SIZE * CHUNK_SIZE) as usize];
pub type EachCell3<T> = [T; (CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE) as usize];

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
}

impl Summary for ChunkSummary {
    fn alloc() -> Box<ChunkSummary> {
        Box::new(ChunkSummary {
            ds_levels: unsafe { mem::zeroed() },
            cave_nums: unsafe { mem::zeroed() },
            cave_connectivity: Vec::new(),
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


pub struct CacheEntry<T> {
    data: Box<T>,
    dirty: bool,
}

impl<T> CacheEntry<T> {
    fn new(data: Box<T>) -> CacheEntry<T> {
        CacheEntry {
            data: data,
            dirty: false,
        }
    }
}

pub struct Cache<'d, T: Summary> {
    storage: &'d Storage,
    name: &'static str,
    cache: LinkedHashMap<(Stable<PlaneId>, V2), CacheEntry<T>>,
}

const CACHE_LIMIT: usize = 1024;

impl<'d, T: Summary> Cache<'d, T> {
    pub fn new(storage: &'d Storage, name: &'static str) -> Cache<'d, T> {
        Cache {
            storage: storage,
            name: name,
            cache: LinkedHashMap::new(),
        }
    }

    fn make_space(&mut self, extra: usize) {
        assert!(extra <= CACHE_LIMIT);
        while self.cache.len() + extra > CACHE_LIMIT {
            let ((pid, cpos), entry) = self.cache.pop_front().unwrap();
            if entry.dirty {
                let file = self.storage.create_summary_file(self.name, pid, cpos);
                warn_on_err!(entry.data.write_to(file));
            }
        }
    }

    pub fn create(&mut self, pid: Stable<PlaneId>, cpos: V2) -> &mut T {
        self.make_space(1);
        self.cache.insert((pid, cpos), CacheEntry::new(T::alloc()));
        self.get_mut(pid, cpos)
    }

    pub fn load(&mut self, pid: Stable<PlaneId>, cpos: V2) -> io::Result<()> {
        if let Some(_) = self.cache.get_refresh(&(pid, cpos)) {
            // Already in the cache.
            Ok(())
        } else {
            self.make_space(1);
            let path = self.storage.summary_file_path(self.name, pid, cpos);
            let file = try!(File::open(path));
            let summary = try!(T::read_from(file));
            self.cache.insert((pid, cpos), CacheEntry::new(summary));
            Ok(())
        }
    }

    // No explicit `unload` - data is unloaded automatically in LRU fashion.

    pub fn get(&self, pid: Stable<PlaneId>, cpos: V2) -> &T {
        &self.cache[&(pid, cpos)].data
    }

    pub fn get_mut(&mut self, pid: Stable<PlaneId>, cpos: V2) -> &mut T {
        let entry = &mut self.cache[&(pid, cpos)];
        entry.dirty = true;
        &mut entry.data
    }
}

impl<'d, T: Summary> Drop for Cache<'d, T> {
    fn drop(&mut self) {
        // Evict everything.
        self.make_space(CACHE_LIMIT);
    }
}
