use std::error::Error;
use std::fs::File;
use std::io;
use linked_hash_map::LinkedHashMap;

use libserver_types::*;
use libserver_config::Storage;


pub trait Summary {
    /// Create a new, empty summary.
    fn alloc() -> Box<Self>;

    /// Write the summary data to a file.
    fn write_to(&self, f: File) -> io::Result<()>;

    /// Create a new summary from the contents of a file.
    fn read_from(f: File) -> io::Result<Box<Self>>;
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
                match entry.data.write_to(file) {
                    Ok(_) => {},
                    Err(e) => {
                        warn!("error writing cache entry to disk: {}",
                              e.description());
                    },
                }
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
