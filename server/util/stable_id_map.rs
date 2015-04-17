use std::collections::HashMap;
use std::marker::PhantomData;
use std::ops::{Index, IndexMut};

use types::{ClientId, EntityId, InventoryId, PlaneId, TerrainChunkId, StructureId};
use types::StableId;
use util::id_map::{self, IdMap};
use util::StrResult;


pub struct StableIdMap<K, V> {
    map: IdMap<V>,
    stable_ids: HashMap<StableId, K>,
    next_id: StableId,
    _marker0: PhantomData<K>,
}

#[derive(Copy, PartialEq, Eq, Debug, Hash)]
pub struct Stable<Id> {
    pub val: StableId,
    _marker0: PhantomData<Id>,
}

impl<Id> Stable<Id> {
    pub fn none() -> Stable<Id> {
        Stable {
            val: NO_STABLE_ID,
            _marker0: PhantomData,
        }
    }

    pub fn new(val: StableId) -> Stable<Id> {
        Stable {
            val: val,
            _marker0: PhantomData,
        }
    }
}

pub const NO_STABLE_ID: StableId = 0;

pub trait IntrusiveStableId {
    fn get_stable_id(&self) -> StableId;
    fn set_stable_id(&mut self, StableId);
}

macro_rules! impl_IntrusiveStableId {
    ($ty:ty, $field:ident) => {
        impl $crate::util::stable_id_map::IntrusiveStableId for $ty {
            fn get_stable_id(&self) -> StableId {
                self.$field
            }

            fn set_stable_id(&mut self, stable_id: StableId) {
                self.$field = stable_id;
            }
        }
    };
}

impl<K: Copy+ObjectId, V: IntrusiveStableId> StableIdMap<K, V> {
    pub fn new() -> StableIdMap<K, V> {
        StableIdMap::new_starting_from(1)
    }

    pub fn new_starting_from(next_id: StableId) -> StableIdMap<K, V> {
        assert!(next_id > 0);
        StableIdMap {
            map: IdMap::new(),
            stable_ids: HashMap::new(),
            next_id: next_id,
            _marker0: PhantomData,
        }
    }

    pub fn next_id(&self) -> StableId {
        self.next_id
    }

    pub fn set_next_id(&mut self, next_id: StableId) {
        assert!(next_id > 0);
        self.next_id = next_id;
    }

    pub fn insert(&mut self, v: V) -> Option<K> {
        let stable_id = v.get_stable_id();
        if stable_id != NO_STABLE_ID && self.stable_ids.contains_key(&stable_id) {
            return None;
        }

        let raw_transient_id = self.map.insert(v);
        let transient_id = ObjectId::from_usize(raw_transient_id);

        if stable_id != NO_STABLE_ID {
            self.stable_ids.insert(stable_id, transient_id);
        }

        Some(transient_id)
    }

    pub fn remove(&mut self, transient_id: K) -> Option<V> {
        let raw_transient_id = transient_id.to_usize();
        let opt_val = self.map.remove(raw_transient_id);

        if let Some(ref val) = opt_val {
            let stable_id = val.get_stable_id();
            if stable_id != NO_STABLE_ID {
                self.stable_ids.remove(&stable_id);
            }
        }

        opt_val
    }

    pub fn set_stable_id(&mut self, transient_id: K, stable_id: StableId) -> StrResult<()> {
        let raw_transient_id = transient_id.to_usize();
        let v = match self.map.get_mut(raw_transient_id) {
            Some(x) => x,
            None => fail!("transient_id is not present in the map"),
        };
        if v.get_stable_id() != NO_STABLE_ID {
            fail!("value already has a stable_id");
        }
        if self.stable_ids.contains_key(&stable_id) {
            fail!("stable_id is already in use");
        }

        if stable_id != NO_STABLE_ID {
            v.set_stable_id(stable_id);
            self.stable_ids.insert(stable_id, transient_id);
        }
        Ok(())
    }

    pub fn pin(&mut self, transient_id: K) -> Stable<K> {
        let raw_transient_id = transient_id.to_usize();
        let val = &mut self.map[raw_transient_id];

        if val.get_stable_id() != NO_STABLE_ID {
            return Stable::new(val.get_stable_id());
        }

        let stable_id = self.next_id;
        self.next_id += 1;

        val.set_stable_id(stable_id);
        self.stable_ids.insert(stable_id, transient_id);

        Stable::new(stable_id)
    }

    pub fn unpin(&mut self, transient_id: K) {
        let raw_transient_id = transient_id.to_usize();
        let val = &mut self.map[raw_transient_id];

        let stable_id = val.get_stable_id();
        if stable_id == NO_STABLE_ID {
            return;
        }

        val.set_stable_id(NO_STABLE_ID);
        self.stable_ids.remove(&stable_id);
    }

    pub fn get_id(&self, stable_id: Stable<K>) -> Option<K> {
        let stable_id = stable_id.val;
        self.stable_ids.get(&stable_id).map(|&x| x)
    }

    pub fn get(&self, transient_id: K) -> Option<&V> {
        self.map.get(transient_id.to_usize())
    }

    pub fn get_mut(&mut self, transient_id: K) -> Option<&mut V> {
        self.map.get_mut(transient_id.to_usize())
    }

    pub fn get_stable(&self, stable_id: Stable<K>) -> Option<&V> {
        self.get_id(stable_id).and_then(|k| self.get(k))
    }

    pub fn get_stable_mut(&mut self, stable_id: Stable<K>) -> Option<&mut V> {
        let k = unwrap_or!(self.get_id(stable_id), return None);
        self.get_mut(k)
    }

    pub fn iter(&self) -> Iter<K, V> {
        Iter {
            iter: self.map.iter(),
            _marker0: PhantomData,
        }
    }
}

impl<K, V> Index<K> for StableIdMap<K, V>
        where K: Copy+ObjectId,
              V: IntrusiveStableId {
    type Output = V;

    fn index(&self, index: &K) -> &V {
        self.get(*index).expect("no entry found for key")
    }
}

impl<K, V> IndexMut<K> for StableIdMap<K, V>
        where K: Copy+ObjectId,
              V: IntrusiveStableId {
    fn index_mut(&mut self, index: &K) -> &mut V {
        self.get_mut(*index).expect("no entry found for key")
    }
}

pub struct Iter<'a, K: 'a, V: 'a> {
    iter: id_map::Iter<'a, V>,
    _marker0: PhantomData<&'a K>,
}

impl<'a, K: ObjectId, V> Iterator for Iter<'a, K, V> {
    type Item = (K, &'a V);
    fn next(&mut self) -> Option<(K, &'a V)> {
        self.iter.next().map(|(k,v)| (ObjectId::from_usize(k), v))
    }
}


pub trait ObjectId: Sized {
    fn to_usize(self) -> usize;
    fn from_usize(x: usize) -> Self;
}

macro_rules! ObjectId_impl {
    ($ty:ident, $inner_ty:ident) => {
        impl ObjectId for $ty {
            fn to_usize(self) -> usize {
                self.0 as usize
            }

            fn from_usize(x: usize) -> $ty {
                $ty(x as $inner_ty)
            }
        }
    };
}

ObjectId_impl!(ClientId, u16);
ObjectId_impl!(EntityId, u32);
ObjectId_impl!(InventoryId, u32);
ObjectId_impl!(PlaneId, u32);
ObjectId_impl!(TerrainChunkId, u32);
ObjectId_impl!(StructureId, u32);
