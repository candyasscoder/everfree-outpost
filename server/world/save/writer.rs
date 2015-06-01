use std::collections::HashMap;
use std::io;
use std::mem;
use std::slice;

use util::{Bytes, Convert};

use super::Result;
use super::SaveId;
use super::{AnyId, ToAnyId};
use super::padding;


pub trait Writer {
    fn write_id<T: ToAnyId>(&mut self, id: T) -> Result<()>;
    fn write_opt_id<T: ToAnyId>(&mut self, opt_id: Option<T>) -> Result<()>;
    fn write_count(&mut self, count: usize) -> Result<()>;
    fn write_bytes(&mut self, buf: &[u8]) -> Result<()>;
    fn write<T: Bytes>(&mut self, x: T) -> Result<()>;

    fn write_str_bytes(&mut self, s: &str) -> Result<()> {
        self.write_bytes(s.as_bytes())
    }

    fn write_str(&mut self, s: &str) -> Result<()> {
        try!(self.write_count(s.len()));
        self.write_str_bytes(s)
    }
}

pub struct WriterWrapper<W: io::Write> {
    writer: W,
    id_map: HashMap<AnyId, SaveId>,
    next_id: SaveId,
}

impl<W: io::Write> WriterWrapper<W> {
    pub fn new(writer: W) -> WriterWrapper<W> {
        WriterWrapper {
            writer: writer,
            id_map: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn id_map(&self) -> &HashMap<AnyId, SaveId> {
        &self.id_map
    }

    fn write_padding(&mut self, len: usize) -> Result<()> {
        let pad = padding(len);
        if pad > 0 {
            try!(self.writer.write_all(&[0; 3][..pad]));
        }
        Ok(())
    }
}

impl<W: io::Write> Writer for WriterWrapper<W> {
    fn write_id<T: ToAnyId>(&mut self, id: T) -> Result<()> {
        use std::collections::hash_map::Entry::{Occupied, Vacant};
        let id = id.to_any_id();
        let save_id = match self.id_map.entry(id) {
            Occupied(e) => *e.get(),
            Vacant(e) => {
                let sid = e.insert(self.next_id);
                self.next_id += 1;
                *sid
            },
        };
        try!(self.write::<u32>(save_id));
        Ok(())
    }

    fn write_opt_id<T: ToAnyId>(&mut self, opt_id: Option<T>) -> Result<()> {
        match opt_id {
            Some(id) => try!(self.write_id(id)),
            None => try!(self.write::<u32>(-1_i32 as SaveId)),
        }
        Ok(())
    }

    fn write_count(&mut self, count: usize) -> Result<()> {
        try!(self.write::<u32>(unwrap!(count.to_u32())));
        Ok(())
    }

    fn write_bytes(&mut self, buf: &[u8]) -> Result<()> {
        try!(self.writer.write_all(buf));
        try!(self.write_padding(buf.len()));
        Ok(())
    }

    fn write<T: Bytes>(&mut self, x: T) -> Result<()> {
        let len = mem::size_of::<T>();

        let buf = unsafe {
            slice::from_raw_parts(&x as *const T as *const u8, len)
        };
        try!(self.writer.write_all(buf));
        try!(self.write_padding(len));
        Ok(())
    }
}
