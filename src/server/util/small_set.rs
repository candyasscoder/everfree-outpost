use std::collections::hash_set::{self, HashSet};
use std::hash::Hash;
use std::marker::PhantomData;
use std::mem;
use std::ptr;
use std::slice;


const SMALL_SET_WORDS: usize = 5;

type Storage = [u64; SMALL_SET_WORDS];

#[unsafe_no_drop_flag]
pub struct SmallSet<T: Eq+Hash> {
    len: usize,
    data: [u64; SMALL_SET_WORDS],
    _marker0: PhantomData<T>,
}

fn small_limit<T>() -> usize {
    SMALL_SET_WORDS * mem::size_of::<u64>() / mem::size_of::<T>()
}

impl<T: Eq+Hash> SmallSet<T> {
    pub fn new() -> SmallSet<T> {
        let result: SmallSet<T> = unsafe { mem::zeroed() };
        assert!(mem::size_of_val(&result.data) >= mem::size_of::<HashSet<T>>());
        assert!(mem::align_of_val(&result.data) >= mem::align_of::<HashSet<T>>());
        result
    }

    pub fn len(&self) -> usize {
        self.len
    }

    fn is_small(&self) -> bool {
        self.len <= small_limit::<T>()
    }

    pub fn insert(&mut self, val: T) -> bool {
        if self.is_small() {
            if unsafe { self.contains_small(&val) } {
                return false;
            }
            if self.len < small_limit::<T>() {
                unsafe { self.insert_small(val) };
            } else {
                unsafe { self.insert_and_grow(val) };
            }
            true
        } else {
            let inserted = unsafe { self.large_mut().insert(val) };
            if inserted {
                self.len += 1;
            }
            inserted
        }
    }

    pub fn remove(&mut self, val: &T) -> bool {
        if self.is_small() {
            unsafe { self.remove_small(val) }
        } else {
            let removed = unsafe { self.large_mut().remove(val) };
            if removed {
                self.len -= 1;
                if self.len <= small_limit::<T>() {
                    unsafe { self.shrink() };
                }
            }
            removed
        }
    }

    pub fn contains(&self, val: &T) -> bool {
        if self.is_small() {
            unsafe { self.contains_small(val) }
        } else {
            unsafe { self.large().contains(val) }
        }
    }

    fn base_ptr(&self) -> *const T {
        &self.data as *const _ as *const T
    }

    // Assumes that there is at least one slot free, and that 'val' is not already present in the
    // set.
    unsafe fn insert_small(&mut self, val: T) {
        let base = self.base_ptr() as *mut T;
        let ptr = base.offset(self.len as isize);
        ptr::write(ptr, val);
        self.len += 1;
    }

    // Assumes that there are no free slots in 'data' and that 'val' is not already in the set.
    unsafe fn insert_and_grow(&mut self, val: T) {
        let base = self.base_ptr();

        let mut large = HashSet::with_capacity(self.len + 1);
        large.insert(val);
        for i in 0..self.len {
            large.insert(ptr::read(base.offset(i as isize)));
        }

        ptr::write(base as *mut HashSet<T>, large);
        self.len += 1;
    }

    // Assumes that 'val' is present in the set.
    unsafe fn remove_small(&mut self, val: &T) -> bool {
        let base = self.base_ptr() as *mut T;
        // self.len > 0 because 'val' is in the set.
        let last = base.offset(self.len as isize - 1);
        for i in 0..self.len {
            let ptr = base.offset(i as isize);
            if *val == *ptr {
                mem::swap(&mut *ptr, &mut *last);
                ptr::read(last);
                self.len -= 1;
                return true;
            }
        }
        false
    }

    unsafe fn contains_small(&self, val: &T) -> bool {
        let base = self.base_ptr();
        for i in 0..self.len {
            let ptr = base.offset(i as isize);
            if *val == *ptr {
                return true;
            }
        }
        false
    }

    // Assumes the set is in large representation but has len == small_limit.
    unsafe fn shrink(&mut self) {
        let base = self.base_ptr() as *mut T;
        let large = ptr::read(base as *const HashSet<T>);
        for (i, val) in large.into_iter().enumerate() {
            let ptr = base.offset(i as isize);
            ptr::write(ptr, val);
        }
    }

    unsafe fn large(&self) -> &HashSet<T> {
        mem::transmute(self.base_ptr())
    }

    unsafe fn large_mut(&mut self) -> &mut HashSet<T> {
        mem::transmute(self.base_ptr())
    }

    pub fn iter(&self) -> Iter<T> {
        if self.is_small() {
            let slice: &[T] = unsafe {
                slice::from_raw_parts(self.base_ptr() as *const T, self.len)
            };
            Iter::Small(slice.iter())
        } else {
            unsafe { Iter::Large(self.large().iter()) }
        }
    }
}

impl<T: Eq+Hash> Drop for SmallSet<T> {
    fn drop(&mut self) {
        if self.is_small() {
            let base = self.base_ptr();
            for i in 0..self.len {
                unsafe { ptr::read(base.offset(i as isize)) };
            }
        } else {
            unsafe { ptr::read(self.base_ptr() as *const HashSet<T>) };
        }
        self.len = 0;
    }
}


pub enum Iter<'a, T: 'a> {
    Small(slice::Iter<'a, T>),
    Large(hash_set::Iter<'a, T>),
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<&'a T> {
        match *self {
            Iter::Small(ref mut iter) => iter.next(),
            Iter::Large(ref mut iter) => iter.next(),
        }
    }
}
