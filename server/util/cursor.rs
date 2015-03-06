use std::mem;
use std::ops::{Deref, DerefMut};

pub struct Cursor<T, P, S>
        where P: DerefMut,
              S: Fn(&mut <P as Deref>::Target) -> &mut T {
    ptr: *mut T,
    parent: P,
    step: S,
}

impl<'a, T1, T2, S> Cursor<T2, &'a mut T1, S>
        where S: Fn(&mut T1) -> &mut T2 {
    pub fn new(root: &'a mut T1, step: S) -> Cursor<T2, &'a mut T1, S> {
        let ptr = step(root) as *mut T2;
        Cursor {
            ptr: ptr,
            parent: root,
            step: step,
        }
    }

    // NB: It is NOT safe to define a general-purpose 'new' for arbitrary parent types.  We rely
    // for safety on the assumption that 'self.parent.deref_mut()' produces a reference to
    // something that strictly outlives the Cursor.  But using a custom DerefMut instance as the
    // parent can return a reference to something with the same lifetime as the parent.  This is a
    // problem because 'new' and 'extend' need to move the parent after calling 'step' on
    // 'parent.deref_mut()', which invalidates pointers into 'parent' itself.
    //
    // Note that even if 'new' didn't move 'parent' itself, the resulting 'Cursor' could also be
    // moved arbitrarily by the caller.
}

impl<T, P, S> Cursor<T, P, S>
        where P: DerefMut,
              S: Fn(&mut <P as Deref>::Target) -> &mut T {
    pub fn extend<T2, S2>(mut self, step: S2) -> Cursor<T2, Cursor<T, P, S>, S2>
            where S2: Fn(&mut T) -> &mut T2 {
        let ptr = step(self.deref_mut()) as *mut T2;
        Cursor {
            ptr: ptr,
            parent: self,
            step: step,
        }
    }

    pub fn up<'a>(&'a mut self) -> ParentRef<'a, T, P, S> {
        ParentRef { owner: self }
    }

    pub fn unwrap(self) -> P {
        self.parent
    }
}

impl<T, P, S> Deref for Cursor<T, P, S>
        where P: DerefMut,
              S: Fn(&mut <P as Deref>::Target) -> &mut T {
    type Target = T;

    fn deref<'a>(&'a self) -> &'a T {
        unsafe { mem::transmute(self.ptr) }
    }
}

impl<T, P, S> DerefMut for Cursor<T, P, S>
        where P: DerefMut,
              S: Fn(&mut <P as Deref>::Target) -> &mut T {
    fn deref_mut<'a>(&'a mut self) -> &'a mut T {
        unsafe { mem::transmute(self.ptr) }
    }
}


pub struct ParentRef<'a, T, P, S>
        where T: 'a,
              P: 'a + DerefMut,
              <P as Deref>::Target: 'a,
              S: 'a + Fn(&mut <P as Deref>::Target) -> &mut T {
    owner: &'a mut Cursor<T, P, S>,
}

#[unsafe_destructor]
impl<'a, T, P, S> Drop for ParentRef<'a, T, P, S>
        where T: 'a,
              P: 'a+DerefMut,
              <P as Deref>::Target: 'a,
              S: 'a+Fn(&mut <P as Deref>::Target) -> &mut T {
    fn drop(&mut self) {
        let o = &mut self.owner;
        o.ptr = (o.step)(o.parent.deref_mut());
    }
}

impl<'a, T, P, S> Deref for ParentRef<'a, T, P, S>
        where T: 'a,
              P: 'a+DerefMut,
              <P as Deref>::Target: 'a,
              S: 'a+Fn(&mut <P as Deref>::Target) -> &mut T {
    type Target = P;

    fn deref<'b>(&'b self) -> &'b P {
        &self.owner.parent
    }
}

impl<'a, T, P, S> DerefMut for ParentRef<'a, T, P, S>
        where T: 'a,
              P: 'a+DerefMut,
              <P as Deref>::Target: 'a,
              S: 'a+Fn(&mut <P as Deref>::Target) -> &mut T {
    fn deref_mut<'b>(&'b mut self) -> &'b mut P {
        &mut self.owner.parent
    }
}
