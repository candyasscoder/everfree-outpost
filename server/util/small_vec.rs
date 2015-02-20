use std::mem;
use std::ptr;
use std::raw;


const SMALL_VEC_WORDS: usize = 3;

type Storage = [u64; SMALL_VEC_WORDS];

#[unsafe_no_drop_flag]
pub struct SmallVec<T> {
    len: usize,
    data: [u64; SMALL_VEC_WORDS],
}

struct SmallVecInterp<T> {
    ptr: *mut T,
    len: usize,
    cap: usize,
    is_large: bool,
}

fn small_limit<T>() -> usize {
    SMALL_VEC_WORDS * mem::size_of::<u64>() / mem::size_of::<T>()
}

impl<T> SmallVec<T> {
    unsafe fn to_interp(&self) -> SmallVecInterp<T> {
        if self.len <= small_limit::<T>() {
            SmallVecInterp {
                ptr: &self.data as *const _ as *mut T,
                len: self.len,
                cap: small_limit::<T>(),
                is_large: false,
            }
        } else {
            SmallVecInterp {
                ptr: self.data[0] as *mut T,
                len: self.len,
                cap: self.data[1] as usize,
                is_large: true,
            }
        }
    }

    unsafe fn from_interp(&mut self, interp: SmallVecInterp<T>) {
        if !interp.is_large {
            assert!(interp.len <= small_limit::<T>());
            self.len = interp.len;
            // Nothing else to do.  self.data was updated in-place.
        } else {
            assert!(interp.len > small_limit::<T>());
            self.len = interp.len;
            self.data[0] = interp.ptr as u64;
            self.data[1] = interp.cap as u64;
        }
    }

    pub fn new() -> SmallVec<T> {
        unsafe { mem::zeroed() }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn capacity(&self) -> usize {
        unsafe { self.to_interp().cap }
    }

    pub fn push(&mut self, val: T) {
        if self.len() == small_limit::<T>() {
            self.to_large_push(val);
        } else {
            let mut interp = unsafe { self.to_interp() };
            interp.push(val);
            unsafe { self.from_interp(interp) };
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.len() == small_limit::<T>() + 1 {
            Some(self.to_small_pop())
        } else {
            let mut interp = unsafe { self.to_interp() };
            let result = interp.pop();
            unsafe { self.from_interp(interp) };
            result
        }
    }

    pub fn as_ptr(&self) -> *const T {
        unsafe { self.to_interp().ptr as *const T }
    }

    pub fn as_mut_ptr(&mut self) -> *mut T {
        unsafe { self.to_interp().ptr }
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

    fn to_large_push(&mut self, val: T) {
        let mut v = Vec::with_capacity(self.len() * 2 + 1);

        let mut interp = unsafe { self.to_interp() };
        for i in range(0, interp.len) {
            let val = unsafe { ptr::read(interp.ptr.offset(i as isize)) };
            v.push(val);
        }
        v.push(val);

        interp.is_large = true;
        unsafe { interp.from_vec(v) };
        unsafe { self.from_interp(interp) };
    }

    fn to_small_pop(&mut self) -> T {
        let mut interp = unsafe { self.to_interp() };
        let mut v = unsafe { interp.to_vec() };

        interp.len = 0;
        interp.is_large = false;

        let result = v.pop().unwrap();
        for val in v.into_iter() {
            interp.push(val);
        }

        unsafe { self.from_interp(interp) };
        result
    }
}

#[unsafe_destructor]
impl<T> Drop for SmallVec<T> {
    fn drop(&mut self) {
        let mut interp = unsafe { self.to_interp() };
        if !interp.is_large {
            for i in range(0, interp.len) {
                unsafe { ptr::read(interp.ptr.offset(i as isize)) };
            }
        } else {
            unsafe { interp.to_vec() };
        }
        self.len = 0;
    }
}


impl<T> SmallVecInterp<T> {
    #[inline]
    fn push(&mut self, val: T) {
        if !self.is_large {
            assert!(self.len < self.cap);
            unsafe { ptr::write(self.ptr.offset(self.len as isize), val); }
            self.len += 1;
        } else {
            let mut v = unsafe { self.to_vec() };
            v.push(val);
            unsafe { self.from_vec(v) };
        }
    }

    #[inline]
    fn pop(&mut self) -> Option<T> {
        if !self.is_large {
            if self.len == 0 {
                None
            } else {
                self.len -= 1;
                Some(unsafe { ptr::read(self.ptr.offset(self.len as isize)) })
            }
        } else {
            assert!(self.len >= small_limit::<T>());
            let mut v = unsafe { self.to_vec() };
            let result = v.pop();
            unsafe { self.from_vec(v) };
            result
        }
    }

    #[inline]
    unsafe fn to_vec(&mut self) -> Vec<T> {
        let vec = mem::transmute((self.ptr, self.len, self.cap));
        self.len = 0;
        self.ptr = ptr::null_mut();
        self.cap = 0;
        vec
    }

    #[inline]
    unsafe fn from_vec(&mut self, v: Vec<T>) {
        let (ptr, len, cap) = mem::transmute(v);
        self.ptr = ptr;
        self.len = len;
        self.cap = cap;
    }
}
