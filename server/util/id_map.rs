use std::collections::BTreeSet;
use std::ops::{Index, IndexMut};
use std::slice;


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

    pub fn iter(&self) -> Iter<V> {
        Iter {
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

pub struct Iter<'a, V: 'a> {
    idx: usize,
    iter: slice::Iter<'a, Option<V>>,
}

impl<'a, V> Iterator for Iter<'a, V> {
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
