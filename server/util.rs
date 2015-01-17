use std::collections::HashMap;
use std::hash::Hash;
use std::collections::hash_map::Hasher;


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
