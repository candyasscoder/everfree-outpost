use std::cmp;
use std::io::IoResult;
use std::mem;


pub struct WireReader<R> {
    r: R,
    msg_left: uint,
}

impl<R: Reader> WireReader<R> {
    pub fn new(r: R) -> WireReader<R> {
        WireReader {
            r: r,
            msg_left: 0,
        }
    }

    pub fn read_header(&mut self) -> IoResult<u16> {
        if !self.done() {
            self.skip_remaining();
        }

        let id: u16 = try!(ReadFrom::read_from(&mut self.r, 2));
        let len: u16 = try!(ReadFrom::read_from(&mut self.r, 2));
        self.msg_left = len as uint;
        Ok(id)
    }

    pub fn read<A: ReadFrom>(&mut self) -> IoResult<A> {
        let result = try!(ReadFrom::read_from(&mut self.r, self.msg_left));

        let (fixed, step) = ReadFrom::size(None::<A>);
        if step == 0 {
            self.msg_left -= fixed;
        } else {
            self.msg_left = (self.msg_left - fixed) % step;
        }

        Ok(result)
    }

    pub fn done(&self) -> bool {
        self.msg_left == 0
    }

    pub fn skip_remaining(&mut self) -> IoResult<()> {
        let mut buf = [0, ..1024];
        while self.msg_left > 0 {
            let count = cmp::min(buf.len(), self.msg_left);
            try!(self.r.read_at_least(count, buf.as_mut_slice()));
            self.msg_left -= count;
        }
        Ok(())
    }
}


pub struct WireWriter<W> {
    w: W,
}

impl<W: Writer> WireWriter<W> {
    pub fn new(w: W) -> WireWriter<W> {
        WireWriter {
            w: w,
        }
    }

    pub fn write_msg<A: WriteTo>(&mut self, id: u16, msg: A) -> IoResult<()> {
        assert!(msg.size() <= ::std::u16::MAX as uint);
        try!(id.write_to(&mut self.w));
        try!((msg.size() as u16).write_to(&mut self.w));
        try!(msg.write_to(&mut self.w));
        Ok(())
    }

    pub fn flush(&mut self) -> IoResult<()> {
        self.w.flush()
    }
}



pub trait ReadFrom {
    fn read_from<R: Reader>(r: &mut R, bytes: uint) -> IoResult<Self>;
    fn size(_: Option<Self>) -> (uint, uint);
}

pub trait ReadFromFixed: ReadFrom {
    fn size_fixed(x: Option<Self>) -> uint {
        let (fixed, step) = ReadFrom::size(x);
        assert!(step == 0);
        fixed
    }
}

pub trait WriteTo {
    fn write_to<W: Writer>(&self, w: &mut W) -> IoResult<()>;
    fn size(&self) -> uint;
}

pub trait WriteToFixed: WriteTo {
    fn size_fixed(_: Option<Self>) -> uint;
}


macro_rules! prim_impl {
    ( $ty:ty, $read_fn:ident, $write_fn:ident ) => {
        impl ReadFrom for $ty {
            #[inline]
            fn read_from<R: Reader>(r: &mut R, bytes: uint) -> IoResult<$ty> {
                assert!(bytes >= mem::size_of::<$ty>());
                r.$read_fn()
            }

            #[inline]
            fn size(_: Option<$ty>) -> (uint, uint) { (mem::size_of::<$ty>(), 0) }
        }

        impl ReadFromFixed for $ty { }

        impl WriteTo for $ty {
            #[inline]
            fn write_to<W: Writer>(&self, w: &mut W) -> IoResult<()> {
                w.$write_fn(*self)
            }

            #[inline]
            fn size(&self) -> uint { mem::size_of::<$ty>() }
        }

        impl WriteToFixed for $ty {
            #[inline]
            fn size_fixed(_: Option<$ty>) -> uint { mem::size_of::<$ty>() }
        }
    }
}

prim_impl!(u8, read_u8, write_u8)
prim_impl!(i8, read_i8, write_i8)
prim_impl!(u16, read_le_u16, write_le_u16)
prim_impl!(i16, read_le_i16, write_le_i16)
prim_impl!(u32, read_le_u32, write_le_u32)
prim_impl!(i32, read_le_i32, write_le_i32)
prim_impl!(u64, read_le_u64, write_le_u64)
prim_impl!(i64, read_le_i64, write_le_i64)
prim_impl!(uint, read_le_uint, write_le_uint)
prim_impl!(int, read_le_int, write_le_int)


impl ReadFrom for () {
    #[inline]
    fn read_from<R: Reader>(_: &mut R, _: uint) -> IoResult<()> {
        Ok(())
    }

    #[inline]
    fn size(_: Option<()>) -> (uint, uint) { (0, 0) }
}

impl ReadFromFixed for () { }

impl WriteTo for () {
    #[inline]
    fn write_to<W: Writer>(&self, _: &mut W) -> IoResult<()> {
        Ok(())
    }

    #[inline]
    fn size(&self) -> uint { 0 }
}

impl WriteToFixed for () {
    #[inline]
    fn size_fixed(_: Option<()>) -> uint { 0 }
}


macro_rules! tuple_impl {
    ( $($name:ident : $ty:ident),+ ; $name1:ident : $ty1:ident  ) => {
        impl<$($ty: ReadFromFixed),+, $ty1: ReadFrom> ReadFrom for ($($ty),+, $ty1) {
            fn read_from<R: Reader>(r: &mut R, bytes: uint) -> IoResult<($($ty),+, $ty1)> {
                let fixed_sum = $(ReadFromFixed::size_fixed(None::<$ty>) +)+ 0;
                $( let $name: $ty = try!(
                        ReadFrom::read_from(r, ReadFromFixed::size_fixed(None::<$ty>))); )+
                let $name1: $ty1 = try!(ReadFrom::read_from(r, bytes - fixed_sum));
                Ok(($($name),+, $name1))
            }

            fn size(_: Option<($($ty),+, $ty1)>) -> (uint, uint) {
                let (fixed1, step1) = ReadFrom::size(None::<$ty1>);
                let fixed = $(ReadFromFixed::size_fixed(None::<$ty>) +)+ fixed1;
                (fixed, step1)
            }
        }

        impl<$($ty: ReadFromFixed),+, $ty1: ReadFromFixed> ReadFromFixed for ($($ty),+, $ty1) { }

        impl<$($ty: WriteTo),+, $ty1: WriteTo> WriteTo for ($($ty),+, $ty1) {
            fn write_to<W: Writer>(&self, w: &mut W) -> IoResult<()> {
                let ($(ref $name),+, ref $name1) = *self;
                $( try!($name.write_to(w)); )*
                try!($name1.write_to(w));
                Ok(())
            }

            fn size(&self) -> uint {
                let ($(ref $name),+, ref $name1) = *self;
                $( $name.size() + )+ $name1.size()
            }
        }

        impl<$($ty: WriteToFixed),+, $ty1: WriteToFixed> WriteToFixed for ($($ty),+, $ty1) {
            fn size_fixed(_: Option<($($ty),+, $ty1)>) -> uint {
                $( WriteToFixed::size_fixed(None::<$ty>) + )+
                    WriteToFixed::size_fixed(None::<$ty1>)
            }
        }
    }
}

tuple_impl!(a: A ; b: B)
tuple_impl!(a: A , b: B ; c: C)
tuple_impl!(a: A , b: B , c: C ; d: D)
tuple_impl!(a: A , b: B , c: C , d: D ; e: E)


impl<A: ReadFromFixed> ReadFrom for Vec<A> {
    fn read_from<R: Reader>(r: &mut R, bytes: uint) -> IoResult<Vec<A>> {
        let step = ReadFromFixed::size_fixed(None::<A>);
        let count = bytes / step;
        let mut result = Vec::with_capacity(count);
        for _ in range(0, count) {
            result.push(try!(ReadFrom::read_from(r, step)));
        }
        Ok(result)
    }

    fn size(_: Option<Vec<A>>) -> (uint, uint) {
        (0, ReadFromFixed::size_fixed(None::<A>))
    }
}

impl<A: WriteToFixed> WriteTo for Vec<A> {
    fn write_to<W: Writer>(&self, w: &mut W) -> IoResult<()> {
        for x in self.iter() {
            try!(x.write_to(w));
        }
        Ok(())
    }

    fn size(&self) -> uint {
        self.len() * WriteToFixed::size_fixed(None::<A>)
    }
}


impl<'a, A: WriteToFixed> WriteTo for &'a [A] {
    fn write_to<W: Writer>(&self, w: &mut W) -> IoResult<()> {
        for x in self.iter() {
            try!(x.write_to(w));
        }
        Ok(())
    }

    fn size(&self) -> uint {
        self.len() * WriteToFixed::size_fixed(None::<A>)
    }
}


impl<'a, A: WriteTo> WriteTo for &'a A {
    fn write_to<W: Writer>(&self, w: &mut W) -> IoResult<()> {
        (*self).write_to(w)
    }

    fn size(&self) -> uint { (*self).size() }
}

impl<'a, A: WriteToFixed> WriteToFixed for &'a A {
    fn size_fixed(_: Option<&'a A>) -> uint {
        WriteToFixed::size_fixed(None::<A>)
    }
}
