use std::collections::BTreeSet;
use std::collections::HashMap;
use std::error::Error;
use std::hash::Hash;
use std::num::{FromPrimitive, ToPrimitive};
use std::collections::hash_map::Hasher;
use std::ops::{Index, IndexMut};
use std::slice;

use types::StableId;


pub struct Refcounted<T> {
    data: T,
    ref_count: u32,
}

impl<T> Refcounted<T> {
    pub fn new(data: T) -> Refcounted<T> {
        Refcounted {
            data: data,
            ref_count: 1,
        }
    }

    pub fn retain(&mut self) {
        self.ref_count += 1;
    }

    pub fn release(&mut self) -> bool {
        self.ref_count -= 1;
        self.ref_count == 0
    }

    pub fn unwrap(self) -> T {
        self.data
    }

    pub fn data(&self) -> &T {
        &self.data
    }

    pub fn data_mut(&mut self) -> &mut T {
        &mut self.data
    }
}


pub struct RefcountedMap<K, V> {
    map: HashMap<K, Refcounted<V>>,
}

impl<K: Eq+Hash<Hasher>, V> RefcountedMap<K, V> {
    pub fn new() -> RefcountedMap<K, V> {
        RefcountedMap {
            map: HashMap::new(),
        }
    }

    pub fn retain<F>(&mut self,
                     key: K,
                     create: F) -> (&mut V, bool)
            where F: FnOnce() -> V {
        use std::collections::hash_map::Entry::*;

        match self.map.entry(key) {
            Vacant(e) => {
                let value = create();
                (e.insert(Refcounted::new(value)).data_mut(), true)
            },
            Occupied(e) => {
                let rc = e.into_mut();
                rc.retain();
                (rc.data_mut(), false)
            },
        }
    }

    pub fn release<F>(&mut self,
                      key: K,
                      destroy: F) -> bool
            where F: FnOnce(V) {
        use std::collections::hash_map::Entry::*;

        match self.map.entry(key) {
            Vacant(_) => {
                panic!("can't release() a nonexistent entry");
            },
            Occupied(mut e) => {
                if e.get_mut().release() {
                    destroy(e.remove().unwrap());
                    true
                } else {
                    false
                }
            },
        }
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.map.get(key).map(|rc| rc.data())
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        self.map.get_mut(key).map(|rc| rc.data_mut())
    }
}


pub struct IdMap<V> {
    map: Vec<Option<V>>,
    free: BTreeSet<usize>,
}

impl<V> IdMap<V> {
    pub fn new() -> IdMap<V> {
        IdMap {
            map: Vec::new(),
            free: BTreeSet::new(),
        }
    }

    fn compact(&mut self) {
        for idx in range(0, self.map.len()).rev() {
            if self.map[idx].is_some() {
                break;
            }
            self.map.pop();
            self.free.remove(&idx);
        }
    }

    pub fn insert(&mut self, v: V) -> usize {
        if self.free.is_empty() {
            let k = self.map.len();
            self.map.push(Some(v));
            k
        } else {
            // nth(0) must be Some because the container is non-empty.
            let &k = self.free.iter().nth(0).unwrap();
            self.free.remove(&k);
            debug_assert!(self.map[k].is_none());
            self.map[k] = Some(v);
            k
        }
    }

    pub fn remove(&mut self, k: usize) -> Option<V> {
        let result = self.map[k].take();
        if result.is_some() {
            if k == self.map.len() - 1 {
                self.map.pop();
                self.compact();
            } else {
                self.free.insert(k);
            }
        }
        result
    }

    pub fn get(&self, k: usize) -> Option<&V> {
        // get(k) returns Option<&Option<V>>.  The outer is None if `k` is out of bounds, and the
        // inner is None if `k` is in bounds but unoccupied.
        match self.map.get(k) {
            None => None,
            Some(&None) => None,
            Some(&Some(ref v)) => Some(v),
        }
    }

    pub fn get_mut(&mut self, k: usize) -> Option<&mut V> {
        match self.map.get_mut(k) {
            None => None,
            Some(&mut None) => None,
            Some(&mut Some(ref mut v)) => Some(v),
        }
    }

    pub fn iter(&self) -> IdMapIter<V> {
        IdMapIter {
            idx: 0,
            iter: self.map.iter(),
        }
    }
}

impl<V> Index<usize> for IdMap<V> {
    type Output = V;

    fn index(&self, index: &usize) -> &V {
        self.get(*index).expect("no entry found for key")
    }
}

impl<V> IndexMut<usize> for IdMap<V> {
    type Output = V;

    fn index_mut(&mut self, index: &usize) -> &mut V {
        self.get_mut(*index).expect("no entry found for key")
    }
}

struct IdMapIter<'a, V: 'a> {
    idx: usize,
    iter: slice::Iter<'a, Option<V>>,
}

impl<'a, V> Iterator for IdMapIter<'a, V> {
    type Item = (usize, &'a V);
    fn next(&mut self) -> Option<(usize, &'a V)> {
        let mut result = None;
        for opt_ref in self.iter {
            self.idx += 1;
            if let Some(ref x) = *opt_ref {
                result = Some((self.idx - 1, x));
                break;
            }
        }
        result
    }
}


pub struct StableIdMap<K, V> {
    map: IdMap<V>,
    stable_ids: HashMap<StableId, K>,
    next_id: StableId,
}

#[derive(Copy, PartialEq, Eq, Show)]
pub struct Stable<Id>(pub StableId);

pub const NO_STABLE_ID: StableId = 0;

pub trait IntrusiveStableId {
    fn get_stable_id(&self) -> StableId;
    fn set_stable_id(&mut self, StableId);
}

macro_rules! impl_IntrusiveStableId {
    ($ty:ty, $field:ident) => {
        impl $crate::util::IntrusiveStableId for $ty {
            fn get_stable_id(&self) -> StableId {
                self.$field
            }

            fn set_stable_id(&mut self, stable_id: StableId) {
                self.$field = stable_id;
            }
        }
    };
}

impl<K: Copy+FromPrimitive+ToPrimitive, V: IntrusiveStableId> StableIdMap<K, V> {
    pub fn new() -> StableIdMap<K, V> {
        StableIdMap::new_starting_from(1)
    }

    pub fn new_starting_from(next_id: StableId) -> StableIdMap<K, V> {
        assert!(next_id > 0);
        StableIdMap {
            map: IdMap::new(),
            stable_ids: HashMap::new(),
            next_id: next_id,
        }
    }

    pub fn insert(&mut self, v: V) -> K {
        let stable_id = v.get_stable_id();

        let raw_transient_id = self.map.insert(v);
        let transient_id = FromPrimitive::from_uint(raw_transient_id).unwrap();

        if stable_id != NO_STABLE_ID {
            self.stable_ids.insert(stable_id, transient_id);
        }

        transient_id
    }

    pub fn remove(&mut self, transient_id: K) -> Option<V> {
        let raw_transient_id = transient_id.to_uint().unwrap();
        let opt_val = self.map.remove(raw_transient_id);

        if let Some(ref val) = opt_val {
            let stable_id = val.get_stable_id();
            if stable_id != NO_STABLE_ID {
                self.stable_ids.remove(&stable_id);
            }
        }

        opt_val
    }

    pub fn pin(&mut self, transient_id: K) -> Stable<K> {
        let raw_transient_id = transient_id.to_uint().unwrap();
        let val = &mut self.map[raw_transient_id];

        if val.get_stable_id() != NO_STABLE_ID {
            return Stable(val.get_stable_id());
        }

        let stable_id = self.next_id;
        self.next_id += 1;

        val.set_stable_id(stable_id);
        self.stable_ids.insert(stable_id, transient_id);

        Stable(stable_id)
    }

    pub fn unpin(&mut self, transient_id: K) {
        let raw_transient_id = transient_id.to_uint().unwrap();
        let val = &mut self.map[raw_transient_id];

        let stable_id = val.get_stable_id();
        if stable_id == NO_STABLE_ID {
            return;
        }

        val.set_stable_id(NO_STABLE_ID);
        self.stable_ids.remove(&stable_id);
    }

    pub fn get_id(&self, stable_id: Stable<K>) -> Option<K> {
        let Stable(stable_id) = stable_id;
        self.stable_ids.get(&stable_id).map(|&x| x)
    }

    pub fn get(&self, transient_id: K) -> Option<&V> {
        self.map.get(transient_id.to_uint().unwrap())
    }

    pub fn get_mut(&mut self, transient_id: K) -> Option<&mut V> {
        self.map.get_mut(transient_id.to_uint().unwrap())
    }

    pub fn iter(&self) -> StableIdMapIter<K, V> {
        StableIdMapIter {
            iter: self.map.iter(),
        }
    }
}

impl<K, V> Index<K> for StableIdMap<K, V>
        where K: Copy+FromPrimitive+ToPrimitive,
              V: IntrusiveStableId {
    type Output = V;

    fn index(&self, index: &K) -> &V {
        self.get(*index).expect("no entry found for key")
    }
}

impl<K, V> IndexMut<K> for StableIdMap<K, V>
        where K: Copy+FromPrimitive+ToPrimitive,
              V: IntrusiveStableId {
    type Output = V;

    fn index_mut(&mut self, index: &K) -> &mut V {
        self.get_mut(*index).expect("no entry found for key")
    }
}

struct StableIdMapIter<'a, K, V: 'a> {
    iter: IdMapIter<'a, V>,
}

impl<'a, K: FromPrimitive, V> Iterator for StableIdMapIter<'a, K, V> {
    type Item = (K, &'a V);
    fn next(&mut self) -> Option<(K, &'a V)> {
        self.iter.next().map(|(k,v)| (FromPrimitive::from_uint(k).unwrap(), v))
    }
}


#[derive(Copy, Show)]
pub struct StrError {
    pub msg: &'static str,
}

impl Error for StrError {
    fn description(&self) -> &'static str {
        self.msg
    }
}

macro_rules! fail {
    ($msg:expr) => {{
            let error = $crate::util::StrError { msg: $msg };
            return Err(::std::error::FromError::from_error(error));
    }};
}

macro_rules! unwrap {
    ($e:expr, $msg:expr) => {
        match $e {
            Some(x) => x,
            None => fail!($msg),
        }
    };
    ($e:expr) => {
        unwrap!($e,
                concat!(file!(), ":", stringify!(line!()),
                ": `", stringify!($e), "` produced `None`"))
    };
}
