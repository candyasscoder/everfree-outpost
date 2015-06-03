use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::io;
use time;

use types::Time;

pub use self::bytes::Bytes;
pub use self::convert::Convert;
pub use self::cursor::Cursor;
pub use self::id_map::IdMap;
pub use self::refcount::RefcountedMap;
pub use self::small_vec::SmallVec;
pub use self::small_set::SmallSet;
pub use self::stable_id_map::{StableIdMap, IntrusiveStableId, Stable};
pub use self::str_error::{StrError, StrResult};
pub use self::str_error::{StringError, StringResult};

#[macro_use] pub mod str_error;
pub mod bytes;
pub mod convert;
pub mod cursor;
pub mod id_map;
pub mod refcount;
pub mod small_set;
pub mod small_vec;
#[macro_use] pub mod stable_id_map;


pub fn multimap_insert<K, V>(map: &mut HashMap<K, HashSet<V>>, k: K, v: V)
        where K: Hash+Eq,
              V: Hash+Eq {
    use std::collections::hash_map::Entry::*;
    let bucket = match map.entry(k) {
        Vacant(e) => e.insert(HashSet::new()),
        Occupied(e) => e.into_mut(),
    };
    bucket.insert(v);
}

pub fn multimap_remove<K, V>(map: &mut HashMap<K, HashSet<V>>, k: K, v: V)
        where K: Hash+Eq,
              V: Hash+Eq {
    use std::collections::hash_map::Entry::*;
    match map.entry(k) {
        Vacant(_) => { },
        Occupied(mut e) => {
            e.get_mut().remove(&v);
            if e.get().is_empty() {
                e.remove();
            }
        },
    }
}


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

macro_rules! unwrap_or {
    ($e:expr, $or:expr) => {
        match $e {
            Some(x) => x,
            None => $or,
        }
    };
    ($e:expr) => { unwrap_or!($e, return) };
}


pub struct OptionIter<I>(Option<I>);

impl<I: Iterator> Iterator for OptionIter<I> {
    type Item = <I as Iterator>::Item;

    fn next(&mut self) -> Option<<I as Iterator>::Item> {
        match self.0 {
            Some(ref mut iter) => iter.next(),
            None => None,
        }
    }
}

pub trait OptionIterExt<I> {
    fn unwrap_iter(self) -> OptionIter<I>;
}

impl<I: Iterator> OptionIterExt<I> for Option<I> {
    fn unwrap_iter(self) -> OptionIter<I> {
        OptionIter(self)
    }
}


pub fn encode_rle16<I: Iterator<Item=u16>>(iter: I) -> Vec<u16> {
    let mut result = Vec::new();

    let mut iter = iter.peekable();
    while !iter.peek().is_none() {
        let cur = iter.next().unwrap();

        // TODO: check that count doesn't overflow 12 bits.
        let mut count = 1u16;
        while count < 0x0fff && iter.peek().map(|&x| x) == Some(cur) {
            iter.next();
            count += 1;
        }
        if count > 1 {
            result.push(0xf000 | count);
        }
        result.push(cur);
    }

    result
}


pub fn now() -> Time {
    let timespec = time::get_time();
    (timespec.sec as Time * 1000) + (timespec.nsec / 1000000) as Time
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
