use std::cmp::max;
use std::mem;
use std::ptr;
use std::raw;
use std::vec;


const SMALL_VEC_WORDS: usize = 4;

type Storage = [u64; SMALL_VEC_WORDS];

#[unsafe_no_drop_flag]
pub struct SmallVec<T> {
    data: Storage,
}

impl<T> SmallVec<T> {
    fn small(&self) -> &SmallView<T> {
        assert!(self.is_small());
        unsafe { mem::transmute(self) }
    }

    fn small_mut(&mut self) -> &mut SmallView<T> {
        assert!(self.is_small());
        unsafe { mem::transmute(self) }
    }

    fn large(&self) -> &LargeView<T> {
        assert!(!self.is_small());
        unsafe { mem::transmute(self) }
    }

    fn large_mut(&mut self) -> &mut LargeView<T> {
        assert!(!self.is_small());
        unsafe { mem::transmute(self) }
    }


    fn is_small(&self) -> bool {
        self.len() <= self.small_limit()
    }

    fn small_limit(&self) -> usize {
        self.small().capacity()
    }


    pub fn new() -> SmallVec<T> {
        unsafe { mem::zeroed() }
    }

    pub fn len(&self) -> usize {
        self.small().len()
    }

    pub fn capacity(&self) -> usize {
        if self.is_small() {
            self.small().capacity()
        } else {
            self.large().capacity()
        }
    }

    pub fn push(&mut self, val: T) {
        if self.is_small() {
            if self.len() < self.small_limit() {
                self.small_mut().push(val);
            } else {
                self.to_large_push(val);
            }
        } else {
            self.large_mut().push(val);
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.is_small() {
            self.small_mut().pop()
        } else {
            if self.len() - 1 > self.small_limit() {
                self.large_mut().pop()
            } else {
                Some(self.to_small_pop())
            }
        }
    }

    pub fn as_ptr(&self) -> *const T {
        if self.is_small() {
            self.small().as_ptr()
        } else {
            self.large().as_ptr()
        }
    }

    pub fn as_mut_ptr(&mut self) -> *mut T {
        if self.is_small() {
            self.small_mut().as_mut_ptr()
        } else {
            self.large_mut().as_mut_ptr()
        }
    }

    pub fn as_slice(&self) -> &[T] {
        unsafe {
            mem::transmute(raw::Slice {
                data: self.as_ptr(),
                len: self.len(),
            })
        }
    }

    pub fn as_mut_slice(&mut self) -> &mut [T] {
        unsafe {
            mem::transmute(raw::Slice {
                data: self.as_ptr(),
                len: self.len(),
            })
        }
    }

    pub fn into_iter(mut self) -> IntoIter<T> {
        if self.is_small() {
            IntoIter::Small(self, 0)
        } else {
            let vec = self.large_mut().to_vec();
            IntoIter::Large(vec.into_iter())
        }
    }

    fn to_large_push(&mut self, val: T) {
        let mut v = Vec::with_capacity(self.len() * 2 + 1);

        for i in range(0, self.len()) {
            let val = unsafe {
                let ptr = self.small().as_ptr().offset(i as isize);
                ptr::read(ptr)
            };
            v.push(val);
        }
        v.push(val);

        unsafe {
            let large: &mut LargeView<T> = mem::transmute(self);
            large.ptr = ptr::null_mut();
            large.from_vec(v);
        }
    }

    fn to_small_pop(&mut self) -> T {
        let mut v = self.large_mut().to_vec();
        let last = v.pop().unwrap();

        assert!(self.len() == 0);   // to_vec ensures this
        for val in v.into_iter() {
            self.push(val);
        }

        last
    }
}

#[unsafe_destructor]
impl<T> Drop for SmallVec<T> {
    fn drop(&mut self) {
        if self.is_small() {
            for i in range(0, self.len()) {
                unsafe {
                    let ptr = self.small().as_ptr().offset(i as isize);
                    drop(ptr::read(ptr));
                }
            }
        } else {
            drop(self.large_mut().to_vec());
        }
        self.large_mut().len = 0;
    }
}


pub enum IntoIter<T> {
    Small(SmallVec<T>, usize),
    Large(vec::IntoIter<T>),
}

impl<T> Iterator for IntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        match *self {
            IntoIter::Small(ref mut vec, ref mut index) => {
                if *index == vec.len() {
                    // Prevent the SmallVec destructor from doing anything.
                    vec.large_mut().len = 0;
                    None
                } else {
                    let val = unsafe { ptr::read(vec.as_ptr().offset(*index as isize)) };
                    *index += 1;
                    Some(val)
                }
            },
            IntoIter::Large(ref mut iter) => {
                iter.next()
            }
        }
    }
}

#[unsafe_destructor]
impl<T> Drop for IntoIter<T> {
    fn drop(&mut self) {
        for _ in *self { }
    }
}


struct SmallView<T> {
    data: Storage,
}

impl<T> SmallView<T> {
    fn len_ref(&self) -> &usize {
        unsafe { mem::transmute(self) }
    }

    fn len_mut(&mut self) -> &mut usize {
        unsafe { mem::transmute(self) }
    }

    fn base(&self) -> usize {
        let usize_size = mem::size_of::<usize>();
        let t_align = mem::align_of::<T>();
        max(usize_size, t_align)
    }


    fn len(&self) -> usize {
        *self.len_ref()
    }

    fn capacity(&self) -> usize {
        let byte_cap = SMALL_VEC_WORDS * mem::size_of::<u64>();
        (byte_cap - self.base()) / mem::size_of::<T>()
    }

    fn as_ptr(&self) -> *const T {
        (self as *const _ as usize + self.base()) as *const T
    }

    fn as_mut_ptr(&mut self) -> *mut T {
        (self as *mut _ as usize + self.base()) as *mut T
    }

    fn push(&mut self, val: T) {
        assert!(self.len() < self.capacity());

        unsafe {
            let ptr = self.as_mut_ptr().offset(self.len() as isize);
            ptr::write(ptr, val);
            *self.len_mut() += 1;
        }
    }

    fn pop(&mut self) -> Option<T> {
        if self.len() == 0 {
            return None;
        }

        unsafe {
            let ptr = self.as_mut_ptr().offset(self.len() as isize);
            let val = ptr::read(ptr);
            *self.len_mut() -= 1;
            Some(val)
        }
    }
}


struct LargeView<T> {
    len: usize,
    ptr: *mut T,
    cap: usize,
}

impl<T> LargeView<T> {
    fn to_vec(&mut self) -> Vec<T> {
        let vec = unsafe { mem::transmute((self.ptr, self.len, self.cap)) };
        self.len = 0;
        self.ptr = ptr::null_mut();
        self.cap = 0;
        vec
    }

    fn from_vec(&mut self, v: Vec<T>) {
        assert!(self.ptr.is_null());
        let (ptr, len, cap) = unsafe { mem::transmute(v) };
        self.ptr = ptr;
        self.len = len;
        self.cap = cap;
    }


    fn len(&self) -> usize {
        self.len
    }

    fn capacity(&self) -> usize {
        self.cap
    }

    fn as_ptr(&self) -> *const T {
        self.ptr as *const T
    }

    fn as_mut_ptr(&self) -> *mut T {
        self.ptr
    }

    fn push(&mut self, val: T) {
        let mut v = self.to_vec();
        v.push(val);
        self.from_vec(v);
    }

    fn pop(&mut self) -> Option<T> {
        let mut v = self.to_vec();
        let result = v.pop();
        self.from_vec(v);
        result
    }
}
