#![crate_name = "server_util"]
#![feature(
    raw,
    unsafe_no_drop_flag,
    )]

extern crate time;
extern crate server_types as libserver_types;

use std::io;
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
