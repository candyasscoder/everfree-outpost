use libc::c_int;

use lua::LuaState;
use lua::REGISTRY_INDEX;
use lua::ValueType;
use util::StrResult;


/// Trait for obtaining a string representation of the name of a type.  The Lua interface code uses
/// this to provide appropriate error messages for invalid argument types.
pub trait TypeName {
    fn type_name() -> &'static str;
}

pub fn type_name<T: TypeName>() -> &'static str {
    <T as TypeName>::type_name()
}

impl<'a, T: TypeName> TypeName for &'a T {
    fn type_name() -> &'static str {
        <T as TypeName>::type_name()
    }
}

impl<'a> TypeName for &'a str {
    fn type_name() -> &'static str { "string" }
}

macro_rules! impl_type_name {
    ($ty:ty) => {
        impl $crate::script::traits::TypeName for $ty {
            fn type_name() -> &'static str {
                stringify!($ty)
            }
        }
    };
}

impl_type_name!(i32);
impl_type_name!(u32);


/// Trait for obtaining the registry key where a type's metatable is stored.
pub trait MetatableKey {
    fn metatable_key() -> &'static str;
}

pub fn metatable_key<T: MetatableKey>() -> &'static str {
    <T as MetatableKey>::metatable_key()
}

macro_rules! impl_metatable_key {
    ($ty:ty) => {
        impl $crate::script::traits::MetatableKey for $ty {
            fn metatable_key() -> &'static str {
                concat!("outpost_metatable_", stringify!($ty))
            }
        }
    };
}


/// Types that can be read from the Lua stack.
pub trait LuaArg<'a>: TypeName {
    fn check(lua: &mut LuaState, index: c_int) -> bool;
    unsafe fn load(lua: &'a LuaState, index: c_int) -> Self;
}

macro_rules! impl_lua_arg_int {
    ($ty:ty) => {
        impl<'a> LuaArg<'a> for $ty {
            fn check(lua: &mut LuaState, index: c_int) -> bool {
                lua.type_of(index) == ValueType::Number
            }

            unsafe fn load(lua: &LuaState, index: c_int) -> $ty {
                lua.to_integer(index) as $ty
            }
        }
    };
}

impl_lua_arg_int!(i32);
impl_lua_arg_int!(u32);

impl<'a> LuaArg<'a> for &'a str {
    fn check(lua: &mut LuaState, index: c_int) -> bool {
        lua.type_of(index) == ValueType::String
    }

    unsafe fn load(lua: &'a LuaState, index: c_int) -> &'a str {
        lua.to_string(index).unwrap()
    }
}

impl<'a, U: Userdata> LuaArg<'a> for &'a U {
    fn check(lua: &mut LuaState, index: c_int) -> bool {
        lua.get_metatable(index);
        lua.get_field(REGISTRY_INDEX, metatable_key::<U>());
        let ok = lua.raw_equal(-1, -2);
        lua.pop(2);
        ok
    }

    unsafe fn load(lua: &'a LuaState, index: c_int) -> &'a U {
        lua.to_userdata::<U>(index).unwrap()
    }
}


/// Lists of arguments that can be read from the Lua stack.
pub trait LuaArgList<'a>: Sized {
    unsafe fn check(lua: &mut LuaState, func: &'static str);
    unsafe fn unpack(lua: &'a mut LuaState) -> Self;
    fn count() -> c_int;
}

macro_rules! check_type {
    ($lua:expr, $ty:ty, $index:expr, $func:expr) => {
        if !<$ty as LuaArg>::check($lua, $index) {
            $lua.push_string(&*format!("bad argument {} to '{}' ({} expected)",
                                       $index,
                                       $func,
                                       <$ty as TypeName>::type_name()));
            $lua.error();
        }
    };
}

macro_rules! impl_lua_arg_list {
    ($count:expr, $($ty:ident $idx:expr),*) => {
        impl<'a, $($ty: LuaArg<'a>),*> LuaArgList<'a> for ($($ty,)*) {
            unsafe fn check(lua: &mut LuaState, func: &'static str) {
                $( check_type!(lua, $ty, $idx, func); )*
            }

            unsafe fn unpack(lua: &'a mut LuaState) -> ($($ty,)*) {
                ($( <$ty as LuaArg>::load(lua, $idx), )*)
            }

            fn count() -> c_int { $count }
        }
    };
}

impl_lua_arg_list!(1, A 1);
impl_lua_arg_list!(2, A 1, B 2);
impl_lua_arg_list!(3, A 1, B 2, C 3);
impl_lua_arg_list!(4, A 1, B 2, C 3, D 4);
impl_lua_arg_list!(5, A 1, B 2, C 3, D 4, E 5);

impl<'a, A: LuaArg<'a>> LuaArgList<'a> for A {
    unsafe fn check(lua: &mut LuaState, func: &'static str) {
        check_type!(lua, A, 1, func);
    }

    unsafe fn unpack(lua: &'a mut LuaState) -> A {
        <A as LuaArg>::load(lua, 1)
    }

    fn count() -> c_int { 1 }
}

pub unsafe fn check_unpack<'a, T: LuaArgList<'a>>(lua: &'a mut LuaState, func: &'static str) -> T {
    <T as LuaArgList>::check(lua, func);
    <T as LuaArgList>::unpack(lua)
}

pub unsafe fn check_unpack_count<'a, T: LuaArgList<'a>>(lua: &'a mut LuaState,
                                                        func: &'static str) -> (T, c_int) {
    <T as LuaArgList>::check(lua, func);
    (<T as LuaArgList>::unpack(lua), <T as LuaArgList>::count())
}


/// Return types that can be pushed onto the Lua stack.
pub trait LuaReturn {
    fn push_onto(self, lua: &mut LuaState);
}

impl<'a> LuaReturn for &'a str {
    fn push_onto(self, lua: &mut LuaState) {
        lua.push_string(self);
    }
}

impl<U: Userdata> LuaReturn for U {
    fn push_onto(self, lua: &mut LuaState) {
        create_userdata(lua, self);
    }
}

macro_rules! impl_lua_return_int {
    ($ty:ty) => {
        impl LuaReturn for $ty {
            fn push_onto(self, lua: &mut LuaState) {
                lua.push_integer(self as isize);
            }
        }
    };
}

impl_lua_return_int!(u16);
impl_lua_return_int!(u32);
impl_lua_return_int!(i32);

impl<T: LuaReturn> LuaReturn for Option<T> {
    fn push_onto(self, lua: &mut LuaState) {
        match self {
            Some(x) => x.push_onto(lua),
            None => lua.push_nil(),
        }
    }
}


/// Lists of return values that can be pushed onto the Lua stack.
pub trait LuaReturnList {
    fn pack(self, lua: &mut LuaState);
    fn count() -> c_int;
}

#[allow(unused_variables)]
impl LuaReturnList for () {
    fn pack(self, lua: &mut LuaState) { }
    fn count() -> c_int { 0 }
}

impl<A: LuaReturn> LuaReturnList for A {
    fn pack(self, lua: &mut LuaState) {
        self.push_onto(lua);
    }

    fn count() -> c_int { 1 }
}

macro_rules! impl_lua_return_list {
    ($count:expr, $($name:ident),*) => {
        impl<$($name: LuaReturn),*> LuaReturnList for ($($name,)*) {
            #[allow(non_snake_case)]
            fn pack(self, lua: &mut LuaState) {
                let ($($name,)*) = self;
                $( $name.push_onto(lua); )*
            }

            fn count() -> c_int { $count }
        }
    };
}

impl_lua_return_list!(1, A);
impl_lua_return_list!(2, A, B);
impl_lua_return_list!(3, A, B, C);
impl_lua_return_list!(4, A, B, C, D);
impl_lua_return_list!(5, A, B, C, D, E);

impl LuaReturnList for StrResult<()> {
    fn pack(self, lua: &mut LuaState) {
        match self {
            Ok(()) => {
                lua.push_bool(true);
                lua.push_nil();
            },
            Err(e) => {
                lua.push_nil();
                e.msg.push_onto(lua);
            },
        }
    }

    fn count() -> c_int { 2 }
}

impl<T: LuaReturn> LuaReturnList for StrResult<T> {
    fn pack(self, lua: &mut LuaState) {
        match self {
            Ok(x) => {
                x.push_onto(lua);
                lua.push_nil();
            },
            Err(e) => {
                lua.push_nil();
                e.msg.push_onto(lua);
            },
        }
    }

    fn count() -> c_int { 2 }
}

pub fn pack_count<T: LuaReturnList>(lua: &mut LuaState, x: T) -> c_int {
    x.pack(lua);
    <T as LuaReturnList>::count()
}


/// Types that can be passed to Lua as userdata values.
#[allow(unused_variables)]
pub trait Userdata: TypeName+MetatableKey+Copy {
    fn populate_table(lua: &mut LuaState) { }
    fn populate_metatable(lua: &mut LuaState) { }
}

pub fn create_userdata<U: Userdata>(lua: &mut LuaState, u: U) {
    lua.push_userdata(u);
    lua.get_field(REGISTRY_INDEX, metatable_key::<U>());
    lua.set_metatable(-2);
}
