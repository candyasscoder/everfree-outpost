use libc::c_int;

use std::ops::{Add, Sub, Mul, Div, Rem};

use physics::v3::V3;

use lua::{OwnedLuaState, LuaState};
use lua::{GLOBALS_INDEX, REGISTRY_INDEX};


const FFI_CALLBACKS_KEY: &'static str = "outpost_ffi_callbacks";
const FFI_LIB_NAME: &'static str = "outpost_ffi";

const BOOTSTRAP_FILE: &'static str = "bootstrap.lua";

pub struct ScriptEngine {
    owned_lua: OwnedLuaState,
}

impl ScriptEngine {
    pub fn new(script_dir: &Path) -> ScriptEngine {
        // OwnedLuaState::new() should return Err only on out-of-memory.
        let mut owned_lua = OwnedLuaState::new().unwrap();

        {
            let mut lua = owned_lua.get();
            lua.open_libs();

            // Set up the `outpost_ffi` library.
            build_ffi_lib(&mut lua);

            // Stack: outpost_ffi
            lua.get_field(REGISTRY_INDEX, "_LOADED");
            lua.copy(-2);
            // Stack: outpost_ffi, _LOADED, outpost_ffi
            lua.set_field(-2, FFI_LIB_NAME);
            lua.pop(1);

            // Stack: outpost_ffi
            lua.set_field(GLOBALS_INDEX, FFI_LIB_NAME);

            // Run the startup script.
            lua.load_file(&script_dir.join(BOOTSTRAP_FILE)).unwrap();
            lua.pcall(0, 0, 0).unwrap();
        }

        ScriptEngine {
            owned_lua: owned_lua,
        }
    }
}

fn build_ffi_lib(lua: &mut LuaState) {
    lua.push_table();

    build_types_table(lua);
    lua.set_field(-2, "types");
}

// NB: assumes the idxs are negative
fn build_type_table<U: Userdata>(lua: &mut LuaState) {
    lua.push_table();

    lua.push_table();
    // Stack: type, table
    <U as Userdata>::populate_table(lua);

    lua.push_table();
    // Stack: type, table, metatable
    // By default, metatable.__index = table.  This way methods stored in `table` will be available
    // with `ud:method()` syntax.  But we also let the Userdata impl override this behavior.
    lua.copy(-2);
    lua.set_field(-2, "__index");
    <U as Userdata>::populate_metatable(lua);

    // Stack: type, table, metatable
    lua.copy(-1);
    lua.set_field(REGISTRY_INDEX, <U as MetatableKey>::metatable_key());
    lua.set_field(-3, "metatable");
    lua.set_field(-2, "table");
}

macro_rules! mk_build_types_table {
    ($($ty:ty),*) => {
        fn build_types_table(lua: &mut LuaState) {
            lua.push_table();
            $({
                build_type_table::<$ty>(lua);
                lua.set_field(-2, <$ty as TypeName>::type_name());
            })*
        }
    }
}

mk_build_types_table!(V3);


trait TypeName {
    fn type_name() -> &'static str;
}

fn type_name<T: TypeName>() -> &'static str {
    <T as TypeName>::type_name()
}

impl<'a, T: TypeName> TypeName for &'a T {
    fn type_name() -> &'static str {
        <T as TypeName>::type_name()
    }
}

macro_rules! impl_type_name {
    ($ty:ty) => {
        impl TypeName for $ty {
            fn type_name() -> &'static str {
                stringify!($ty)
            }
        }
    };
}

impl_type_name!(i32);
impl_type_name!(V3);


trait MetatableKey {
    fn metatable_key() -> &'static str;
}

fn metatable_key<T: MetatableKey>() -> &'static str {
    <T as MetatableKey>::metatable_key()
}

macro_rules! impl_metatable_key {
    ($ty:ty) => {
        impl MetatableKey for $ty {
            fn metatable_key() -> &'static str {
                concat!("outpost_metatable_", stringify!($ty))
            }
        }
    };
}

impl_metatable_key!(V3);


trait LuaArg<'a>: TypeName {
    fn check(lua: &mut LuaState, index: c_int) -> bool;
    unsafe fn load(lua: &'a LuaState, index: c_int) -> Self;
}

impl<'a> LuaArg<'a> for i32 {
    fn check(lua: &mut LuaState, index: c_int) -> bool { true }

    unsafe fn load(lua: &LuaState, index: c_int) -> i32 {
        lua.to_integer(index) as i32
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


trait LuaArgList<'a>: Sized {
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

unsafe fn check_unpack<'a, T: LuaArgList<'a>>(lua: &'a mut LuaState, func: &'static str) -> T {
    <T as LuaArgList>::check(lua, func);
    <T as LuaArgList>::unpack(lua)
}

unsafe fn check_unpack_count<'a, T: LuaArgList<'a>>(lua: &'a mut LuaState,
                                                    func: &'static str) -> (T, c_int) {
    <T as LuaArgList>::check(lua, func);
    (<T as LuaArgList>::unpack(lua), <T as LuaArgList>::count())
}


trait LuaReturn {
    fn push_onto(self, lua: &mut LuaState);
}

impl<U: Userdata> LuaReturn for U {
    fn push_onto(self, lua: &mut LuaState) {
        create_userdata(lua, self);
    }
}

impl LuaReturn for i32 {
    fn push_onto(self, lua: &mut LuaState) {
        lua.push_integer(self as isize);
    }
}


trait LuaReturnList {
    fn pack(self, lua: &mut LuaState);
    fn count() -> c_int;
}

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

fn pack_count<T: LuaReturnList>(lua: &mut LuaState, x: T) -> c_int {
    x.pack(lua);
    <T as LuaReturnList>::count()
}





fn v3_new(mut lua: LuaState) -> c_int {
    let (x, y, z) = unsafe { check_unpack(&mut lua, "v3_new") };
    lua.pop(3);

    create_userdata(&mut lua, V3::new(x, y, z));
    1
}


macro_rules! field_setter {
    ($fn_name:ident : $ty:ty , $field:ident) => {
        fn $fn_name(mut lua: LuaState) -> c_int {
            let value = {
                let (_ptr, val): (&$ty, _) = unsafe { check_unpack(&mut lua, stringify!($fn_name)) };
                val
            };
            {
                let ptr_mut = unsafe { lua.to_userdata_mut::<$ty>(1).unwrap() };
                ptr_mut.$field = value;
            }
            lua.pop(2);

            0
        }
    };
}

macro_rules! count_pats {
    () => { 0 };
    ($first:pat) => { 1 };
    ($first:pat, $($rest:pat),*) => { 1 + count_pats!($($rest),*) };
}

macro_rules! func_wrapper {
    ($fn_name:ident : $ty:ty, $pat:pat = $pat_ty:ty => $val:expr ) => {
        fn $fn_name(mut lua: LuaState) -> c_int {
            let (value, count) = {
                let ($pat, count): ($pat_ty, c_int) = unsafe {
                    check_unpack_count(&mut lua, stringify!($fn_name))
                };
                ($val, count)
            };
            lua.pop(count);
            pack_count(&mut lua, value)
        }
    };
    ($fn_name:ident : $ty:ty, $pat:pat => $val:expr ) => {
        func_wrapper!($fn_name: $ty, $pat = _ => $val);
    };
}

macro_rules! binop {
    ($fn_name:ident : $ty:ty, $trait_:ident, $method:ident) => {
        func_wrapper!($fn_name: $ty, (a, b) = (&$ty, &$ty) => <$ty as $trait_>::$method(*a, *b));
    };
}

macro_rules! field_getter {
    ($fn_name:ident : $ty:ty , $field:ident) => {
        func_wrapper!($fn_name: $ty, ptr = &$ty => ptr.$field);
    };
}


// NB: These all assume the index $idx is negative
macro_rules! insert_func {
    ($lua:expr, $idx:expr, ($($name:ident)*): $func:expr) => {{
        $lua.push_rust_function($func);
        $lua.set_field($idx - 1, concat!($(stringify!($name)),*));
    }};

    ($lua:expr, $idx:expr, $name:ident: $func:expr) => {
        insert_func!($lua, $idx, ($name): $func);
    };

    ($lua:expr, $idx:expr, $func:ident) => {
        insert_func!($lua, $idx, $func: $func);
    };
}

macro_rules! field_getters {
    ($lua:expr, $idx:expr, $ty:ty, $($field:ident),*) => {{
        $({
            field_getter!($field : $ty, $field);
            insert_func!($lua, $idx, $field);
        })*
    }};
}

macro_rules! field_setters {
    ($lua:expr, $idx:expr, $ty:ty, $($field:ident),*) => {{
        $({
            field_setter!($field : $ty, $field);
            insert_func!($lua, $idx, (set_ $field): $field);
        })*
    }};
}

macro_rules! binops {
    ($lua:expr, $idx:expr, $ty:ty, $($meta:ident = $trait_:ident :: $method:ident),*) => {{
        $({
            binop!($method: $ty, $trait_, $method);
            insert_func!($lua, $idx, $meta: $method);
        })*
    }};
}

macro_rules! static_funcs {
    ($lua:expr, $idx:expr, $ty:ty,
     $( $pat:pat => $ty_name:ident :: $func:ident ($($arg:expr),*) ),*) => {
        $({
            func_wrapper!($func: $ty, $pat => $ty_name::$func ($($arg),*));
            insert_func!($lua, $idx, $func);
        })*
    };
}

macro_rules! methods {
    ($lua:expr, $idx:expr, $ty:ty,
     $( ($($pat:pat),*) => self . $method:ident ($($arg:expr),*) ),*) => {
        $({
            func_wrapper!($method: $ty, (ptr_, $($pat),*) => {
                let ptr: &$ty = ptr_;
                ptr.$method($($arg),*)
            });
            insert_func!($lua, $idx, $method);
        })*
    };
}

macro_rules! funcs {
    ($lua:expr, $idx:expr, $ty:ty,
     $( fn $name:ident ( $($arg:ident : $arg_ty:ty),* ) $body:expr )*) => {
        $({
            func_wrapper!($name: $ty, ($($arg,)*) = ($($arg_ty,)*) => $body);
            insert_func!($lua, $idx, $name);
        })*
    };
}


trait Userdata: TypeName+MetatableKey+Copy {
    fn populate_table(lua: &mut LuaState) { }
    fn populate_metatable(lua: &mut LuaState) { }
}

fn create_userdata<U: Userdata>(lua: &mut LuaState, u: U) {
    lua.push_userdata(u);
    lua.get_field(REGISTRY_INDEX, metatable_key::<U>());
    lua.set_metatable(-2);
}


impl Userdata for V3 {
    fn populate_table(lua: &mut LuaState) {
        field_getters!(lua, -1, V3, x, y, z);
        field_setters!(lua, -1, V3, x, y, z);

        static_funcs!(lua, -1, V3,
                      (x, y, z) => V3::new(x, y, z));
        methods!(lua, -1, V3,
                 () => self.abs());
        funcs!(lua, -1, V3,
               fn extract(ptr: &V3) {
                   (ptr.x, ptr.y, ptr.z)
               });
    }

    fn populate_metatable(lua: &mut LuaState) {
        binops!(lua, -1, V3,
                __add = Add::add,
                __sub = Sub::sub,
                __mul = Mul::mul,
                __div = Div::div,
                __mod = Rem::rem);
    }
}
