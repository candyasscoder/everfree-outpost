use std::cmp;
use std::collections::HashMap;
use std::hash::Hash;
use std::io::{self, Read, Write};
use std::mem;
use std::slice;
use std::u16;

use types::*;


pub struct WireReader<R> {
    r: R,
    msg_left: usize,
}

impl<R: Read> WireReader<R> {
    pub fn new(r: R) -> WireReader<R> {
        WireReader {
            r: r,
            msg_left: 0,
        }
    }

    pub fn read_header(&mut self) -> io::Result<WireId> {
        if !self.done() {
            try!(self.skip_remaining());
        }

        self.msg_left = 4;
        let id = try!(self.read());
        let len = try!(self.read::<u16>());
        self.msg_left = len as usize;
        Ok(id)
    }

    fn read_raw(&mut self, dest: &mut [u8]) -> io::Result<()> {
        if dest.len() > self.msg_left {
            let msg = format!("not enough bytes in message: expected {}, but only {} remain",
                              dest.len(), self.msg_left);
            return Err(io::Error::new(io::ErrorKind::Other, msg));
        }

        let mut base = 0;
        while base < dest.len() {
            let n = try!(self.r.read(&mut dest[base..]));
            assert!(n > 0 && base + n <= dest.len());
            base += n;
        }

        self.msg_left -= dest.len();
        Ok(())
    }

    pub fn read<A: ReadFrom>(&mut self) -> io::Result<A> {
        ReadFrom::read_from(self)
    }

    pub fn done(&self) -> bool {
        self.msg_left == 0
    }

    pub fn skip_remaining(&mut self) -> io::Result<()> {
        let mut buf = [0; 1024];
        while self.msg_left > 0 {
            let count = cmp::min(buf.len(), self.msg_left);
            let n = try!(self.r.read(&mut buf[..count]));
            assert!(n > 0);
            self.msg_left -= n;
        }
        Ok(())
    }
}


pub struct WireWriter<W> {
    w: W,
    msg_left: usize,
}

impl<W: Write> WireWriter<W> {
    pub fn new(w: W) -> WireWriter<W> {
        WireWriter {
            w: w,
            msg_left: 0,
        }
    }

    pub fn write_msg<A: WriteTo>(&mut self, id: WireId, msg: A) -> io::Result<()> {
        // In case an error occurred while writing the previous message, pad it out to the expected
        // length to avoid confusing the destination.  (The message will contain garbage, but at
        // least it will be the right size.)
        try!(self.zero_remaining());

        assert!(msg.size() <= u16::MAX as usize);
        self.msg_left = 4 + msg.size();
        try!(id.write_to(self));
        try!((msg.size() as u16).write_to(self));
        try!(msg.write_to(self));
        Ok(())
    }

    pub fn write<A: WriteTo>(&mut self, msg: A) -> io::Result<()> {
        msg.write_to(self)
    }

    fn write_raw(&mut self, src: &[u8]) -> io::Result<()> {
        if src.len() > self.msg_left {
            let msg = format!("too many bytes in message: expected {}, but tried to write {}",
                              self.msg_left, src.len());
            return Err(io::Error::new(io::ErrorKind::Other, msg));
        }
        try!(self.w.write_all(src));
        self.msg_left -= src.len();
        Ok(())
    }

    fn zero_remaining(&mut self) -> io::Result<()> {
        let buf = [0; 1024];
        while self.msg_left > 0 {
            let count = cmp::min(buf.len(), self.msg_left);
            try!(self.w.write_all(&buf[..count]));
            self.msg_left -= count;
        }
        Ok(())
    }

    pub fn flush(&mut self) -> io::Result<()> {
        self.w.flush()
    }
}



pub trait ReadFrom {
    fn read_from<R: Read>(r: &mut WireReader<R>) -> io::Result<Self>;
}

pub trait WriteTo {
    fn write_to<W: Write>(&self, w: &mut WireWriter<W>) -> io::Result<()>;
    fn size(&self) -> usize;
    fn size_is_fixed() -> bool;
}


macro_rules! prim_impl {
    ( $ty:ty, $read_fn:ident, $write_fn:ident ) => {
        impl ReadFrom for $ty {
            #[inline]
            fn read_from<R: Read>(r: &mut WireReader<R>) -> io::Result<$ty> {
                let mut val: $ty = 0;
                {
                    let buf = unsafe {
                        slice::from_raw_parts_mut(&mut val as *mut $ty as *mut u8,
                                                  mem::size_of::<$ty>())
                    };
                    try!(r.read_raw(buf));
                }
                Ok(val)
            }
        }

        impl WriteTo for $ty {
            #[inline]
            fn write_to<W: Write>(&self, w: &mut WireWriter<W>) -> io::Result<()> {
                let buf = unsafe {
                    slice::from_raw_parts(self as *const $ty as *const u8,
                                          mem::size_of::<$ty>())
                };
                try!(w.write_raw(buf));
                Ok(())
            }

            #[inline]
            fn size(&self) -> usize { mem::size_of::<$ty>() }

            #[inline]
            fn size_is_fixed() -> bool { true }
        }
    }
}

prim_impl!(u8, read_u8, write_u8);
prim_impl!(i8, read_i8, write_i8);
prim_impl!(u16, read_le_u16, write_le_u16);
prim_impl!(i16, read_le_i16, write_le_i16);
prim_impl!(u32, read_le_u32, write_le_u32);
prim_impl!(i32, read_le_i32, write_le_i32);
prim_impl!(u64, read_le_u64, write_le_u64);
prim_impl!(i64, read_le_i64, write_le_i64);


impl ReadFrom for () {
    #[inline]
    fn read_from<R: Read>(_: &mut WireReader<R>) -> io::Result<()> {
        Ok(())
    }
}

impl WriteTo for () {
    #[inline]
    fn write_to<W: Write>(&self, _: &mut WireWriter<W>) -> io::Result<()> {
        Ok(())
    }

    #[inline]
    fn size(&self) -> usize { 0 }

    #[inline]
    fn size_is_fixed() -> bool { true }
}


macro_rules! tuple_impl {
    ( $($name:ident : $ty:ident),+ ) => {
        impl<$($ty: ReadFrom),+> ReadFrom for ($($ty),+) {
            fn read_from<R: Read>(r: &mut WireReader<R>) -> io::Result<($($ty),+)> {
                $( let $name: $ty = try!(ReadFrom::read_from(r)); )+
                Ok(($($name),+))
            }
        }

        impl<$($ty: WriteTo),+> WriteTo for ($($ty),+) {
            fn write_to<W: Write>(&self, w: &mut WireWriter<W>) -> io::Result<()> {
                let ($(ref $name),+) = *self;
                $( try!($name.write_to(w)); )*
                Ok(())
            }

            fn size(&self) -> usize {
                let ($(ref $name),+) = *self;
                0 $( + $name.size() )+
            }

            fn size_is_fixed() -> bool {
                true $( && <$ty as WriteTo>::size_is_fixed() )+
            }
        }
    }
}

tuple_impl!(a: A , b: B);
tuple_impl!(a: A , b: B , c: C);
tuple_impl!(a: A , b: B , c: C , d: D);
tuple_impl!(a: A , b: B , c: C , d: D , e: E);
tuple_impl!(a: A , b: B , c: C , d: D , e: E , f: F);


macro_rules! id_newtype_impl {
    ($name:ident : $inner:ident) => {
        impl ReadFrom for $name {
            #[inline]
            fn read_from<R: Read>(r: &mut WireReader<R>) -> io::Result<$name> {
                <$inner as ReadFrom>::read_from(r)
                    .map(|x| $name(x))
            }
        }

        impl WriteTo for $name {
            #[inline]
            fn write_to<W: Write>(&self, w: &mut WireWriter<W>) -> io::Result<()> {
                self.unwrap().write_to(w)
            }

            #[inline]
            fn size(&self) -> usize { <$inner as WriteTo>::size(&self.unwrap()) }

            #[inline]
            fn size_is_fixed() -> bool { <$inner as WriteTo>::size_is_fixed() }
        }
    };
}

id_newtype_impl!(WireId: u16);
id_newtype_impl!(EntityId: u32);
id_newtype_impl!(StructureId: u32);
id_newtype_impl!(InventoryId: u32);


impl<A: ReadFrom> ReadFrom for Vec<A> {
    fn read_from<R: Read>(r: &mut WireReader<R>) -> io::Result<Vec<A>> {
        let count = try!(r.read::<u16>()) as usize;
        let mut result = Vec::with_capacity(count);
        for _ in 0..count {
            result.push(try!(r.read()));
        }
        Ok(result)
    }
}

impl<A: WriteTo> WriteTo for Vec<A> {
    fn write_to<W: Write>(&self, w: &mut WireWriter<W>) -> io::Result<()> {
        (*self).write_to(w)
    }

    fn size(&self) -> usize {
        (*self).size()
    }

    fn size_is_fixed() -> bool { false }
}


impl<A: WriteTo> WriteTo for [A] {
    fn write_to<W: Write>(&self, w: &mut WireWriter<W>) -> io::Result<()> {
        assert!(self.len() <= u16::MAX as usize);
        try!(w.write(self.len() as u16));
        for x in self.iter() {
            try!(w.write(x));
        }
        Ok(())
    }

    fn size(&self) -> usize {
        if self.len() == 0 {
            123_u16.size()
        } else if <A as WriteTo>::size_is_fixed() {
            123_u16.size() + self.len() * self[0].size()
        } else {
            let mut size = 123_u16.size();
            for x in self.iter() {
                size += x.size();
            }
            size
        }
    }

    fn size_is_fixed() -> bool { false }
}


impl<'a, A: WriteTo> WriteTo for &'a A {
    fn write_to<W: Write>(&self, w: &mut WireWriter<W>) -> io::Result<()> {
        (*self).write_to(w)
    }

    fn size(&self) -> usize { (*self).size() }

    fn size_is_fixed() -> bool { <A as WriteTo>::size_is_fixed() }
}


impl ReadFrom for String {
    fn read_from<R: Read>(r: &mut WireReader<R>) -> io::Result<String> {
        let bytes: Vec<u8> = try!(ReadFrom::read_from(r));
        Ok(String::from_utf8_lossy(&*bytes).into_owned())
    }
}

impl WriteTo for String {
    fn write_to<W: Write>(&self, w: &mut WireWriter<W>) -> io::Result<()> {
        self.as_bytes().write_to(w)
    }

    fn size(&self) -> usize {
        self.as_bytes().size()
    }

    fn size_is_fixed() -> bool { false }
}


impl ReadFrom for [u32; 4] {
    #[inline]
    fn read_from<R: Read>(r: &mut WireReader<R>) -> io::Result<[u32; 4]> {
        let (a, b, c, d) = try!(r.read());
        Ok([a, b, c, d])
    }
}

impl WriteTo for [u32; 4] {
    #[inline]
    fn write_to<W: Write>(&self, w: &mut WireWriter<W>) -> io::Result<()> {
        w.write((self[0], self[1], self[2], self[3]))
    }

    #[inline]
    fn size(&self) -> usize { 16 }

    #[inline]
    fn size_is_fixed() -> bool { true }
}


impl<K: ReadFrom+Eq+Hash, V: ReadFrom> ReadFrom for HashMap<K, V> {
    #[inline]
    fn read_from<R: Read>(r: &mut WireReader<R>) -> io::Result<HashMap<K, V>> {
        let count = try!(r.read::<u16>()) as usize;
        let mut result = HashMap::with_capacity(count);
        for _ in 0..count {
            let k = try!(r.read());
            let v = try!(r.read());
            result.insert(k, v);
        }
        Ok(result)
    }
}

impl<K: WriteTo+Eq+Hash, V: WriteTo> WriteTo for HashMap<K, V> {
    #[inline]
    fn write_to<W: Write>(&self, w: &mut WireWriter<W>) -> io::Result<()> {
        assert!(self.len() <= u16::MAX as usize);
        try!(w.write(self.len() as u16));
        for (k, v) in self.iter() {
            try!(w.write(k));
            try!(w.write(v));
        }
        Ok(())
    }

    #[inline]
    fn size(&self) -> usize {
        let mut size = 123_u16.size();
        for (k, v) in self.iter() {
            size += k.size() + v.size();
        }
        size
    }

    #[inline]
    fn size_is_fixed() -> bool { false }
}
