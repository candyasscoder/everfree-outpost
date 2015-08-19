macro_rules! conv_closure {
    ($old:ident, $new:ident) => {
        |x: $old| {
            let y = x as $new;
            if y as $old == x {
                Some(y)
            } else {
                None
            }
        }
    };
}

pub trait Convert: Sized {
    fn to_u64(self) -> Option<u64>;
    fn to_i64(self) -> Option<i64>;


    #[inline]
    fn to_u8(self) -> Option<u8> {
        self.to_u64().and_then(conv_closure!(u64, u8))
    }

    #[inline]
    fn to_u16(self) -> Option<u16> {
        self.to_u64().and_then(conv_closure!(u64, u16))
    }

    #[inline]
    fn to_u32(self) -> Option<u32> {
        self.to_u64().and_then(conv_closure!(u64, u32))
    }

    #[inline]
    fn to_usize(self) -> Option<usize> {
        self.to_u64().and_then(conv_closure!(u64, usize))
    }


    #[inline]
    fn to_i8(self) -> Option<i8> {
        self.to_i64().and_then(conv_closure!(i64, i8))
    }

    #[inline]
    fn to_i16(self) -> Option<i16> {
        self.to_i64().and_then(conv_closure!(i64, i16))
    }

    #[inline]
    fn to_i32(self) -> Option<i32> {
        self.to_i64().and_then(conv_closure!(i64, i32))
    }

    #[inline]
    fn to_isize(self) -> Option<isize> {
        self.to_i64().and_then(conv_closure!(i64, isize))
    }
}


macro_rules! unsigned_impl {
    ($ty:ident) => {
        impl Convert for $ty {
            fn to_u64(self) -> Option<u64> {
                Some(self as u64)
            }

            fn to_i64(self) -> Option<i64> {
                let i = self as i64;
                if i < 0 {
                    None
                } else {
                    Some(i)
                }
            }
        }
    };
}

unsigned_impl!(u8);
unsigned_impl!(u16);
unsigned_impl!(u32);
unsigned_impl!(u64);
unsigned_impl!(usize);


macro_rules! signed_impl {
    ($ty:ident) => {
        impl Convert for $ty {
            fn to_u64(self) -> Option<u64> {
                if self < 0 {
                    None
                } else {
                    Some(self as u64)
                }
            }

            fn to_i64(self) -> Option<i64> {
                Some(self as i64)
            }
        }
    };
}


signed_impl!(i8);
signed_impl!(i16);
signed_impl!(i32);
signed_impl!(i64);
signed_impl!(isize);
