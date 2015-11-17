use std::collections::{HashMap, HashSet};
use std::io;
use std::iter;
use std::mem;
use std::raw;

use types::*;
use util::{Bytes, Convert};
use world::fragment::Fragment;
use world::ops;

use super::Result;
use super::SaveId;
use super::{AnyId, ToAnyId};
use super::padding;


pub trait Reader {
    fn read_id<'d, T: ReadId, F: Fragment<'d>>(&mut self, f: &mut F) -> Result<T>;
    fn read_opt_id<'d, T: ReadId, F: Fragment<'d>>(&mut self, f: &mut F) -> Result<Option<T>>;
    fn read_count(&mut self) -> Result<usize>;
    fn read_bytes(&mut self, len: usize) -> Result<Vec<u8>>;
    fn read_buf(&mut self, buf: &mut [u8]) -> Result<()>;
    fn read<T: Bytes>(&mut self) -> Result<T>;

    fn read_str_bytes(&mut self, len: usize) -> Result<String> {
        match String::from_utf8(try!(self.read_bytes(len))) {
            Ok(s) => Ok(s),
            Err(_) => fail!("utf8 encoding error"),
        }
    }

    fn read_str(&mut self) -> Result<String> {
        let len = try!(self.read_count());
        self.read_str_bytes(len)
    }
}

pub struct ReaderWrapper<R: io::Read> {
    reader: R,
    id_map: HashMap<SaveId, AnyId>,
    created_objs: HashSet<AnyId>,
}

impl<R: io::Read> ReaderWrapper<R> {
    pub fn new(reader: R) -> ReaderWrapper<R> {
        ReaderWrapper {
            reader: reader,
            id_map: HashMap::new(),
            created_objs: HashSet::new(),
        }
    }

    pub fn id_map(&self) -> &HashMap<SaveId, AnyId> {
        &self.id_map
    }

    pub fn id_map_mut(&mut self) -> &mut HashMap<SaveId, AnyId> {
        &mut self.id_map
    }

    pub fn created_objs(&self) -> &HashSet<AnyId> {
        &self.created_objs
    }

    fn read_id_helper<'d, T: ReadId, F: Fragment<'d>>(&mut self,
                                                      f: &mut F,
                                                      save_id: SaveId) -> Result<T> {
        use std::collections::hash_map::Entry::{Occupied, Vacant};
        match self.id_map.entry(save_id) {
            Occupied(e) => ReadId::from_any_id(*e.get()),
            Vacant(e) => {
                let id = <T as ReadId>::fabricate(f);
                self.created_objs.insert(id.to_any_id());
                e.insert(id.to_any_id());
                Ok(id)
            },
        }
    }
}

impl<R: io::Read> Reader for ReaderWrapper<R> {
    fn read_id<'d, T: ReadId, F: Fragment<'d>>(&mut self, f: &mut F) -> Result<T> {
        let save_id = try!(self.read::<u32>());
        self.read_id_helper(f, save_id)
    }

    fn read_opt_id<'d, T: ReadId, F: Fragment<'d>>(&mut self, f: &mut F) -> Result<Option<T>> {
        let save_id = try!(self.read::<u32>());
        if save_id == -1_i32 as SaveId {
            Ok(None)
        } else {
            let id = try!(self.read_id_helper(f, save_id));
            Ok(Some(id))
        }
    }

    fn read_count(&mut self) -> Result<usize> {
        let count = try!(self.read::<u32>());
        Ok(unwrap!(count.to_usize()))
    }

    fn read_bytes(&mut self, len: usize) -> Result<Vec<u8>> {
        let pad = padding(len);
        let mut vec = iter::repeat(0_u8).take(len + pad).collect::<Vec<_>>();
        try!(self.read_buf(&mut vec));
        vec.truncate(len);
        Ok(vec)
    }

    fn read_buf(&mut self, buf: &mut [u8]) -> Result<()> {
        let mut base = 0;
        while base < buf.len() {
            let n = try!(self.reader.read(&mut buf[base..]));
            assert!(n > 0 && base + n <= buf.len());
            base += n;
        }
        Ok(())
    }

    fn read<T: Bytes>(&mut self) -> Result<T> {
        let len = mem::size_of::<T>();
        let pad = padding(len);

        let result: (T, u32) = unsafe { mem::zeroed() };
        assert!(mem::size_of_val(&result) >= len + pad);
        let buf = unsafe {
            mem::transmute(raw::Slice {
                data: &result as *const (T, u32) as *const u8,
                len: len + pad,
            })
        };
        try!(self.read_buf(buf));
        Ok(result.0)
    }
}


pub trait ReadId: ToAnyId+Copy {
    fn from_any_id(id: AnyId) -> Result<Self>;
    fn fabricate<'d, F: Fragment<'d>>(f: &mut F) -> Self;
}

impl ReadId for ClientId {
    fn from_any_id(id: AnyId) -> Result<ClientId> {
        match id {
            AnyId::Client(id) => Ok(id),
            _ => fail!("expected AnyID::Client"),
        }
    }

    fn fabricate<'d, F: Fragment<'d>>(f: &mut F) -> ClientId {
        ops::client::create_unchecked(f)
    }
}

impl ReadId for EntityId {
    fn from_any_id(id: AnyId) -> Result<EntityId> {
        match id {
            AnyId::Entity(id) => Ok(id),
            _ => fail!("expected AnyID::Entity"),
        }
    }

    fn fabricate<'d, F: Fragment<'d>>(f: &mut F) -> EntityId {
        ops::entity::create_unchecked(f)
    }
}

impl ReadId for InventoryId {
    fn from_any_id(id: AnyId) -> Result<InventoryId> {
        match id {
            AnyId::Inventory(id) => Ok(id),
            _ => fail!("expected AnyID::Inventory"),
        }
    }

    fn fabricate<'d, F: Fragment<'d>>(f: &mut F) -> InventoryId {
        ops::inventory::create_unchecked(f, 0)
    }
}

impl ReadId for PlaneId {
    fn from_any_id(id: AnyId) -> Result<PlaneId> {
        match id {
            AnyId::Plane(id) => Ok(id),
            _ => fail!("expected AnyID::Plane"),
        }
    }

    fn fabricate<'d, F: Fragment<'d>>(f: &mut F) -> PlaneId {
        ops::plane::create_unchecked(f)
    }
}

impl ReadId for TerrainChunkId {
    fn from_any_id(id: AnyId) -> Result<TerrainChunkId> {
        match id {
            AnyId::TerrainChunk(id) => Ok(id),
            _ => fail!("expected AnyID::TerrainChunk"),
        }
    }

    fn fabricate<'d, F: Fragment<'d>>(f: &mut F) -> TerrainChunkId {
        ops::terrain_chunk::create_unchecked(f)
    }
}

impl ReadId for StructureId {
    fn from_any_id(id: AnyId) -> Result<StructureId> {
        match id {
            AnyId::Structure(id) => Ok(id),
            _ => fail!("expected AnyID::Structure"),
        }
    }

    fn fabricate<'d, F: Fragment<'d>>(f: &mut F) -> StructureId {
        ops::structure::create_unchecked(f)
    }
}
