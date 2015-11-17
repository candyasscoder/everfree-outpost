#![crate_name = "server_util"]
#![feature(
    raw,
    unsafe_no_drop_flag,
    )]

extern crate time;
extern crate server_types as libserver_types;

use std::io;
use std::iter;
use std::mem;
use std::raw;

use libserver_types::Time;

pub use self::bit_slice::BitSlice;
pub use self::bytes::Bytes;
pub use self::convert::Convert;
pub use self::small_vec::SmallVec;
pub use self::small_set::SmallSet;
pub use self::str_error::{StrError, StrResult};
pub use self::str_error::{StringError, StringResult};

#[macro_use] pub mod str_error;
pub mod bit_slice;
pub mod bytes;
pub mod convert;
pub mod small_set;
pub mod small_vec;


#[macro_export]
macro_rules! warn_on_err {
    ($e:expr) => {
        match $e {
            Ok(_) => {},
            Err(e) => warn!("{}: {}",
                            stringify!($e),
                            ::std::error::Error::description(&e)),
        }
    };
}


pub fn now() -> Time {
    let timespec = time::get_time();
    (timespec.sec as Time * 1000) + (timespec.nsec / 1000000) as Time
}


pub unsafe fn transmute_slice<'a, T, U>(x: &'a [T]) -> &'a [U] {
    mem::transmute(raw::Slice {
        data: x.as_ptr() as *const U,
        len: x.len() * mem::size_of::<T>() / mem::size_of::<U>(),
    })
}

pub unsafe fn transmute_slice_mut<'a, T, U>(x: &'a mut [T]) -> &'a mut [U] {
    mem::transmute(raw::Slice {
        data: x.as_ptr() as *const U,
        len: x.len() * mem::size_of::<T>() / mem::size_of::<U>(),
    })
}


pub trait ReadExact {
    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()>;
}

impl<R: io::Read> ReadExact for R {
    fn read_exact(&mut self, buf: &mut [u8]) -> io::Result<()> {
        let mut base = 0;
        while base < buf.len() {
            let n = try!(self.read(&mut buf[base..]));
            assert!(n > 0 && base + n <= buf.len());
            base += n;
        }
        Ok(())
    }
}


/// Filter a `Vec<T>` in-place, not preserving order.
pub fn filter_in_place<T, F: FnMut(&T) -> bool>(vec: &mut Vec<T>, mut f: F) {
    let mut i = 0;
    while i < vec.len() {
        if f(&vec[i]) {
            i += 1;
        } else {
            vec.swap_remove(i);
        }
    }
}


pub unsafe fn write_vec<W: io::Write, T>(w: &mut W, v: &Vec<T>) -> io::Result<()> {
    write_array(w, v)
}

pub unsafe fn read_vec<R: io::Read, T>(r: &mut R) -> io::Result<Vec<T>> {
    use self::bytes::ReadBytes;
    let len = try!(r.read_bytes::<u32>()) as usize;
    let mut v = Vec::with_capacity(len);
    v.set_len(len);
    try!(r.read_exact(transmute_slice_mut(&mut v)));
    Ok(v)
}

pub unsafe fn write_array<W: io::Write, T>(w: &mut W, v: &[T]) -> io::Result<()> {
    use self::bytes::WriteBytes;
    try!(w.write_bytes(v.len().to_u32().unwrap()));
    try!(w.write_all(transmute_slice(v)));
    Ok(())
}

pub unsafe fn read_array<R: io::Read, T>(r: &mut R) -> io::Result<Box<[T]>> {
    read_vec(r).map(|v| v.into_boxed_slice())
}


pub fn make_array<T: Copy>(init: T, len: usize) -> Box<[T]> {
    iter::repeat(init).take(len).collect::<Vec<_>>().into_boxed_slice()
}

pub fn make_array_with<T, F: FnMut() -> T>(len: usize, mut f: F) -> Box<[T]> {
    let mut v = Vec::with_capacity(len);
    for _ in 0 .. len {
        v.push(f());
    }
    v.into_boxed_slice()
}

