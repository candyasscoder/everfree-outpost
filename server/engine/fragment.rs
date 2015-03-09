pub trait Open<'a, T> {
    fn open(&'a self) -> T;
}

trait OpenFrom<'a, T> {
    fn open_from(&'a T) -> Self;
}

impl<'a, T, A: OpenFrom<'a, T>, B: OpenFrom<'a, T>> OpenFrom<'a, T> for (A, B) {
    fn open_from(x: &'a T) -> (A, B) {
        (OpenFrom::open_from(x), OpenFrom::open_from(x))
    }
}

impl<'a, T, A: OpenFrom<'a, T>> Open<'a, A> for T {
    fn open(&'a self) -> A {
        OpenFrom::open_from(self)
    }
}


pub trait OpenMut<'a, T> {
    fn open_mut(&'a mut self) -> T;
}

trait OpenMutFrom<'a, T> {
    fn open_mut_from(x: &'a mut T) -> Self;
}

unsafe trait OpenMutFromUnsafe<'a, T> {
    unsafe fn open_mut_from_unsafe(x: *mut T) -> &'a mut Self;
}

impl<'a, T, A: OpenMutFromUnsafe<'a, T>, B: OpenMutFromUnsafe<'a, T>> OpenMutFrom<'a, T> for (&'a mut A, &'a mut B) {
    fn open_mut_from(x: &'a mut T) -> (&'a mut A, &'a mut B) {
        unsafe {
            let a = OpenMutFromUnsafe::open_mut_from_unsafe(x as *mut T);
            let b = OpenMutFromUnsafe::open_mut_from_unsafe(x as *mut T);
            (a, b)
        }
    }
}

impl<'a, T, A: OpenMutFrom<'a, T>> OpenMut<'a, A> for T {
    fn open_mut(&'a mut self) -> A {
        OpenMutFrom::open_mut_from(self)
    }
}


unsafe impl<'a, 'd> OpenMutFromUnsafe<'a, ::engine::Engine<'d>> for ::world::World<'d> {
    unsafe fn open_mut_from_unsafe(x: *mut ::engine::Engine<'d>) -> &'a mut ::world::World<'d> {
        &mut (*x).world
    }
}


/*
impl<'a, T, A: OpenFrom<'a, T>, B: OpenFrom<'a, T>> OpenFrom<'a, T> for (A, B) {
    fn open_from(x: &'a T) -> (A, B) {
        (OpenFrom::open_from(x), OpenFrom::open_from(x))
    }
}
*/



trait Test: for<'a> OpenMut<'a, (&'a mut World, &'a mut World)> {
    fn test(&self) -> &u32 {
        self.open()
    }
}
