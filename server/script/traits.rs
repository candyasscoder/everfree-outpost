use libc::c_int;

use lua::LuaState;
use lua::REGISTRY_INDEX;
use lua::ValueType;
use util::StrResult;

use super::{ScriptContext, get_ctx};


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
pub trait FromLua<'a> {
    unsafe fn check(lua: &mut LuaState, index: c_int, func: &'static str);
    unsafe fn from_lua(lua: &'a LuaState, index: c_int) -> Self;
    fn count() -> c_int { 1 }
}

macro_rules! type_error {
    ($lua:expr, $index:expr, $func:expr, $ty_name:expr) => {{
        $lua.push_string(&*format!("bad argument {} to '{}' ({} expected)",
                                   $index,
                                   $func,
                                   $ty_name));
        $lua.error();
    }};
}

macro_rules! int_from_lua_impl {
    ($ty:ty) => {
        impl<'a> FromLua<'a> for $ty {
            unsafe fn check(lua: &mut LuaState, index: c_int, func: &'static str) {
                if lua.type_of(index) != ValueType::Number {
                    type_error!(lua, index, func, "number");
                }
            }

            unsafe fn from_lua(lua: &LuaState, index: c_int) -> $ty {
                lua.to_integer(index) as $ty
            }
        }
    };
}

int_from_lua_impl!(i32);
int_from_lua_impl!(u32);

impl<'a> FromLua<'a> for &'a str {
    unsafe fn check(lua: &mut LuaState, index: c_int, func: &'static str) {
        if lua.type_of(index) != ValueType::String {
            type_error!(lua, index, func, "string");
        }
    }

    unsafe fn from_lua(lua: &'a LuaState, index: c_int) -> &'a str {
        lua.to_string(index).unwrap()
    }
}

impl<'a, U: Userdata> FromLua<'a> for &'a U {
    unsafe fn check(lua: &mut LuaState, index: c_int, func: &'static str) {
        lua.get_metatable(index);
        lua.get_field(REGISTRY_INDEX, metatable_key::<U>());
        let ok = lua.raw_equal(-1, -2);
        lua.pop(2);

        if !ok {
            type_error!(lua, index, func, type_name::<U>());
        }
    }

    unsafe fn from_lua(lua: &'a LuaState, index: c_int) -> &'a U {
        lua.to_userdata::<U>(index).unwrap()
    }
}

impl<'a, U: Userdata+Copy+'a> FromLua<'a> for U {
    unsafe fn check(lua: &mut LuaState, index: c_int, func: &'static str) {
        <&'a U as FromLua>::check(lua, index, func);
    }

    unsafe fn from_lua(lua: &'a LuaState, index: c_int) -> U {
        let ptr = <&'a U as FromLua>::from_lua(lua, index);
        *ptr
    }
}

macro_rules! tuple_from_lua_impl {
    ($count:expr, $($ty:ident)*) => {
        #[allow(unused_variables, unused_mut, unused_attributes, non_snake_case)]
        impl<'a, $($ty: FromLua<'a>),*> FromLua<'a> for ($($ty,)*) {
            unsafe fn check(lua: &mut LuaState, mut index: c_int, func: &'static str) {
                $(
                    <$ty as FromLua>::check(lua, index, func);
                    index += <$ty as FromLua>::count();
                )*
                let _ = index;  // last value assigned to `index` is never read
            }

            unsafe fn from_lua(lua: &'a LuaState, mut index: c_int) -> ($($ty,)*) {
                $(
                    let $ty: $ty = <$ty as FromLua>::from_lua(lua, index);
                    index += <$ty as FromLua>::count();
                )*
                let _ = index;  // last value assigned to `index` is never read
                ($($ty,)*)
            }

            fn count() -> c_int {
                let mut sum = 0;
                $( sum += <$ty as FromLua>::count(); )*
                sum
            }
        }
    };
}

tuple_from_lua_impl!(0,);
tuple_from_lua_impl!(1, A);
tuple_from_lua_impl!(2, A B);
tuple_from_lua_impl!(3, A B C);
tuple_from_lua_impl!(4, A B C D);
tuple_from_lua_impl!(5, A B C D E);

pub unsafe fn check_args<'a, T: FromLua<'a>>(lua: &mut LuaState, func: &'static str) {
    let actual = lua.top_index();
    let expected = <T as FromLua>::count();
    if actual != expected {
        lua.push_string(&*format!("wrong number of arguments for '{}' ({} expected)",
                                  func,
                                  expected));
        lua.error();
    }

    <T as FromLua>::check(lua, 1, func);
}

pub unsafe fn unpack_args<'a, T: FromLua<'a>>(lua: &'a mut LuaState, func: &'static str) -> T {
    check_args::<T>(lua, func);
    let x = <T as FromLua>::from_lua(lua, 1);
    x
}

pub unsafe fn unpack_args_count<'a, T: FromLua<'a>>(lua: &'a mut LuaState,
                                                    func: &'static str) -> (T, c_int) {
    let x = unpack_args(lua, func);
    (x, <T as FromLua>::count())
}

pub unsafe fn with_args_count_ctx<'a, T, R, F>(lua: &'a mut LuaState,
                                               func: &'static str,
                                               f: F) -> (R, c_int)
        where T: FromLua<'a>,
              F: FnOnce(&mut ScriptContext, T) -> R {

    check_args::<T>(lua, func);

    let mut ctx = get_ctx(lua);
    let count = <T as FromLua>::count();
    let args = <T as FromLua>::from_lua(lua, 1);
    (f(&mut *ctx, args), count)
}


/// Return types that can be pushed onto the Lua stack.
pub trait ToLua {
    fn to_lua(self, lua: &mut LuaState);
    fn count() -> c_int { 1 }
}

macro_rules! int_to_lua_impl {
    ($ty:ty) => {
        impl ToLua for $ty {
            fn to_lua(self, lua: &mut LuaState) {
                lua.push_integer(self as isize);
            }
        }
    };
}

int_to_lua_impl!(u16);
int_to_lua_impl!(u32);
int_to_lua_impl!(i32);

impl<'a> ToLua for &'a str {
    fn to_lua(self, lua: &mut LuaState) {
        lua.push_string(self);
    }
}

impl<U: Userdata> ToLua for U {
    fn to_lua(self, lua: &mut LuaState) {
        create_userdata(lua, self);
    }
}

macro_rules! tuple_to_lua_impl {
    ($count:expr, $($ty:ident)*) => {
        #[allow(unused_variables, unused_mut, unused_attributes, non_snake_case)]
        impl<$($ty: ToLua),*> ToLua for ($($ty,)*) {
            fn to_lua(self, lua: &mut LuaState) {
                let ($($ty,)*): ($($ty,)*) = self;
                $( $ty.to_lua(lua); )*
            }

            fn count() -> c_int {
                let mut sum = 0;
                $( sum += <$ty as ToLua>::count(); )*
                sum
            }
        }
    };
}

tuple_to_lua_impl!(0,);
tuple_to_lua_impl!(1, A);
tuple_to_lua_impl!(2, A B);
tuple_to_lua_impl!(3, A B C);
tuple_to_lua_impl!(4, A B C D);
tuple_to_lua_impl!(5, A B C D E);

impl<T: ToLua> ToLua for Option<T> {
    fn to_lua(self, lua: &mut LuaState) {
        match self {
            Some(x) => x.to_lua(lua),
            None => {
                for _ in range(0, <T as ToLua>::count()) {
                    lua.push_nil();
                }
            }
        }
    }

    fn count() -> c_int { <T as ToLua>::count() }
}

impl<T: ToLua> ToLua for StrResult<T> {
    fn to_lua(self, lua: &mut LuaState) {
        match self {
            Ok(x) => {
                x.to_lua(lua);
                lua.push_nil();
            },
            Err(e) => {
                for _ in range(0, <T as ToLua>::count()) {
                    lua.push_nil();
                }
                e.msg.to_lua(lua);
            },
        }
    }

    fn count() -> c_int { <T as ToLua>::count() + 1 }
}

pub fn pack_count<T: ToLua>(lua: &mut LuaState, x: T) -> c_int {
    x.to_lua(lua);
    <T as ToLua>::count()
}


macro_rules! lua_fn {
    (fn $name:ident($($arg_name:ident : $arg_ty:ty),*) -> $ret_ty:ty { $body:expr }) => {
        fn $name(mut lua: LuaState) -> c_int {
            let (result, count): ($ret_ty, ::libc::c_int) = {
                let (($($arg_name,)*), count): (($($arg_ty,)*), ::libc::c_int) = unsafe {
                    $crate::script::traits::unpack_args_count(&mut lua, stringify!($name))
                };
                ($body, count)
            };
            lua.pop(count);
            $crate::script::traits::pack_count(&mut lua, result)
        }
    };
}

macro_rules! lua_ctx_fn {
    (fn $name:ident($ctx_name:ident, $($arg_name:ident : $arg_ty:ty),*)
            -> $ret_ty:ty { $body:expr }) => {
        fn $name(mut lua: LuaState) -> c_int {
            let (result, count): ($ret_ty, ::libc::c_int) = unsafe {
                $crate::script::traits::with_args_count_ctx(&mut lua, stringify!($name),
                    |$ctx_name, args| {
                        let ($($arg_name,)*): ($($arg_ty,)*) = args;
                        $body
                    })
            };
            lua.pop(count);
            $crate::script::traits::pack_count(&mut lua, result)
        }
    };
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
