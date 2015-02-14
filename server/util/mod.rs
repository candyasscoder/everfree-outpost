use std::collections::{HashMap, HashSet};
use std::collections::hash_map::Hasher;
use std::hash::Hash;

pub use self::bytes::Bytes;
pub use self::cursor::Cursor;
pub use self::id_map::IdMap;
pub use self::refcount::RefcountedMap;
pub use self::stable_id_map::{StableIdMap, IntrusiveStableId};
pub use self::str_error::{StrError, StrResult};

pub mod bytes;
pub mod cursor;
pub mod id_map;
pub mod refcount;
pub mod small_vec;
#[macro_use] pub mod stable_id_map;
#[macro_use] pub mod str_error;


pub fn multimap_insert<K, V>(map: &mut HashMap<K, HashSet<V>>, k: K, v: V)
        where K: Hash<Hasher>+Eq,
              V: Hash<Hasher>+Eq {
    use std::collections::hash_map::Entry::*;
    let bucket = match map.entry(k) {
        Vacant(e) => e.insert(HashSet::new()),
        Occupied(e) => e.into_mut(),
    };
    bucket.insert(v);
}

pub fn multimap_remove<K, V>(map: &mut HashMap<K, HashSet<V>>, k: K, v: V)
        where K: Hash<Hasher>+Eq,
              V: Hash<Hasher>+Eq {
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
