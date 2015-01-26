use std::cell::{RefCell, RefMut};
use std::mem;
use std::ops::{Add, Sub, Mul, Div, Rem};
use libc::c_int;

use physics::CHUNK_SIZE;
use physics::v3::{Vn, V3, scalar};

use state;
use terrain;
use types::{Time, ClientId, EntityId, StructureId};
use input::ActionBits;
use world;
use world::object::*;

use lua::{OwnedLuaState, LuaState};
use lua::{GLOBALS_INDEX, REGISTRY_INDEX};
use lua::ValueType;


const FFI_CALLBACKS_KEY: &'static str = "outpost_ffi_callbacks";
const FFI_LIB_NAME: &'static str = "outpost_ffi";

const CTX_KEY: &'static str = "outpost_ctx";

const BOOTSTRAP_FILE: &'static str = "bootstrap.lua";

macro_rules! callbacks {
    ($($caps_name:ident = $name:expr;)*) => {
        $( const $caps_name: &'static str = concat!("outpost_callback_", $name); )*

        const ALL_CALLBACKS: &'static [(&'static str, &'static str)] = &[
            $( ($name, concat!("outpost_callback_", $name)) ),*
        ];
    };
}

callbacks! {
    CB_KEY_TEST = "test";
}

pub struct ScriptEngine {
    owned_lua: OwnedLuaState,
}

struct ScriptContext<'a, 'd: 'a> {
    world: &'a mut world::World<'d>,
    now: Time,
}

impl<'a, 'd> ScriptContext<'a, 'd> {
    pub fn new(world: &'a mut world::World<'d>, now: Time) -> ScriptContext<'a, 'd> {
        ScriptContext {
            world: world,
            now: now,
        }
    }
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

    fn with_context<F, T>(&mut self,
                          ctx: &RefCell<ScriptContext>,
                          blk: F) -> T
            where F: FnOnce(&mut LuaState) -> T {

        let mut lua = self.owned_lua.get();
        lua.push_light_userdata(ctx as *const _ as *mut RefCell<ScriptContext>);
        lua.set_field(REGISTRY_INDEX, CTX_KEY);

        let x = blk(&mut lua);

        lua.push_nil();
        lua.set_field(REGISTRY_INDEX, CTX_KEY);

        x
    }


    pub fn test_callback(&mut self,
                         world: &mut world::World,
                         now: Time,
                         id: ClientId,
                         action: ActionBits) {
        let ctx = RefCell::new(ScriptContext {
            world: world,
            now: now,
        });
        self.with_context(&ctx, |lua| {
            lua.get_field(REGISTRY_INDEX, "outpost_callback_test");
            let c = Client { id: id };
            c.push_onto(lua);
            lua.pcall(1, 0, 0).unwrap();
        });
    }
}

pub fn get_ctx_ref<'a>(lua: &mut LuaState) -> &'a RefCell<ScriptContext<'a, 'a>> {
    lua.get_field(REGISTRY_INDEX, CTX_KEY);
    let raw_ptr: *mut RefCell<ScriptContext> = unsafe { lua.to_userdata_raw(-1) };
    lua.pop(1);

    if raw_ptr.is_null() {
        panic!("tried to access script context, but no context is active");
    }

    unsafe { mem::transmute(raw_ptr) }
}

pub fn get_ctx<'a>(lua: &mut LuaState) -> RefMut<'a, ScriptContext<'a, 'a>> {
    get_ctx_ref(lua).borrow_mut()
}

fn build_ffi_lib(lua: &mut LuaState) {
    lua.push_table();

    build_types_table(lua);
    lua.set_field(-2, "types");

    build_callbacks_table(lua);
    lua.set_field(-2, "callbacks");
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

mk_build_types_table!(V3, World, Structure, Client, Entity);

fn build_callbacks_table(lua: &mut LuaState) {
    lua.push_table();

    lua.push_table();
    lua.push_rust_function(callbacks_table_newindex);
    lua.set_field(-2, "__newindex");
    lua.push_rust_function(callbacks_table_index);
    lua.set_field(-2, "__index");
    lua.set_metatable(-2);

    for &(base, _) in ALL_CALLBACKS.iter() {
        lua.push_rust_function(lua_no_op);
        lua.set_field(-2, base);
    }
}

fn callbacks_table_newindex(mut lua: LuaState) -> c_int {
    // Stack: table, key, value
    let cb_key = match lua.to_string(-2) {
        None => return 0,
        Some(x) => format!("outpost_callback_{}", x),
    };

    // Store callback into the registry.
    lua.set_field(REGISTRY_INDEX, &*cb_key);

    // Don't write into the actual table.  __index/__newindex are ignored when the field already
    // exists.
    lua.pop(2);
    0
}

fn callbacks_table_index(mut lua: LuaState) -> c_int {
    // Stack: table, key
    let cb_key = match lua.to_string(-1) {
        None => return 0,
        Some(x) => format!("outpost_callback_{}", x),
    };

    // Load callback into the registry.
    lua.get_field(REGISTRY_INDEX, &*cb_key);
    1
}

fn lua_no_op(_: LuaState) -> c_int {
    0
}


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

impl<'a> TypeName for &'a str {
    fn type_name() -> &'static str { "string" }
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
impl_type_name!(u32);


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


trait LuaArg<'a>: TypeName {
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

impl_lua_return_int!(i32);
impl_lua_return_int!(u32);

impl<T: LuaReturn> LuaReturn for Option<T> {
    fn push_onto(self, lua: &mut LuaState) {
        match self {
            Some(x) => x.push_onto(lua),
            None => lua.push_nil(),
        }
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
     $( $pat:pat => $ty_name:ident :: $func:ident ($($arg:expr),*) ),*) => {{
        $({
            func_wrapper!($func: $ty, $pat => $ty_name::$func ($($arg),*));
            insert_func!($lua, $idx, $func);
        })*
    }};
}

macro_rules! methods {
    ($lua:expr, $idx:expr, $ty:ty,
     $( ($($pat:pat),*) => self . $method:ident ($($arg:expr),*) ),*) => {{
        $({
            func_wrapper!($method: $ty, (ptr_, $($pat),*) => {
                let ptr: &$ty = ptr_;
                ptr.$method($($arg),*)
            });
            insert_func!($lua, $idx, $method);
        })*
    }};
}

macro_rules! funcs {
    ($lua:expr, $idx:expr, $ty:ty,
     $( fn $name:ident ( $($arg:ident : $arg_ty:ty),* ) $body:expr )*) => {{
        $({
            func_wrapper!($name: $ty, ($($arg,)*) = ($($arg_ty,)*) => $body);
            insert_func!($lua, $idx, $name);
        })*
    }};
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


impl_type_name!(V3);
impl_metatable_key!(V3);

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



macro_rules! func_wrapper_ctx {
    ($fn_name:ident : $ty:ty, $ctx:ident, $pat:pat = $pat_ty:ty => $val:expr ) => {
        fn $fn_name(mut lua: LuaState) -> c_int {
            // FIXME: ctx_ref is a RefCell borrow, so it has a destructor, but the destructor won't
            // run if lua.error is called!
            let mut ctx_ref = get_ctx(&mut lua);
            let $ctx = &mut *ctx_ref;
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
    ($fn_name:ident : $ty:ty, $ctx:ident, $pat:pat => $val:expr ) => {
        func_wrapper_ctx!($fn_name: $ty, $ctx, $pat = _ => $val);
    };
}

macro_rules! methods_ctx {
    ($lua:expr, $idx:expr, $ty:ty,
     $( ($($pat:pat),*) => self . $method:ident ($($arg:expr),*) ),*) => {{
        $({
            func_wrapper_ctx!($method: $ty, ctx, (ptr_, $($pat),*) => {
                let ptr: &$ty = ptr_;
                ptr.$method(ctx, $($arg),*)
            });
            insert_func!($lua, $idx, $method);
        })*
    }};
}


#[derive(Copy)]
struct World;

impl World {
    fn find_structure_at_point(&self, ctx: &ScriptContext, pos: &V3) -> Option<Structure> {
        let pos = *pos;
        let chunk = pos.reduce().div_floor(scalar(CHUNK_SIZE));
        for s in ctx.world.chunk_structures(chunk) {
            if s.bounds().contains(pos) {
                return Some(Structure { id: s.id() });
            }
        }
        None
    }

    fn create_structure(&self, ctx: &mut ScriptContext, pos: &V3, template_id: u32) -> Option<Structure> {
        ctx.world.create_structure(*pos, template_id)
           .map(|s| Structure { id: s.id() }).ok()
    }
}

impl_type_name!(World);
impl_metatable_key!(World);

impl Userdata for World {
    fn populate_table(lua: &mut LuaState) {
        methods_ctx!(lua, -1, World,
                     (pos) => self.find_structure_at_point(pos),
                     (pos, template_id) => self.create_structure(pos, template_id));
    }
}


#[derive(Copy)]
struct Structure {
    id: StructureId,
}

impl Structure {
    fn pos(&self, ctx: &ScriptContext) -> Option<V3> {
        ctx.world.get_structure(self.id)
           .map(|s| s.pos())
    }

    fn size(&self, ctx: &ScriptContext) -> Option<V3> {
        ctx.world.get_structure(self.id)
           .map(|s| s.size())
    }

    fn template_id(&self, ctx: &ScriptContext) -> Option<u32> {
        ctx.world.get_structure(self.id)
           .map(|s| s.template_id())
    }

    fn template<'a>(&self, ctx: &'a ScriptContext) -> Option<&'a str> {
        self.template_id(ctx)
            .and_then(|id| ctx.world.data().object_templates.get_template(id))
            .map(|t| &*t.name)
    }

    fn delete(&self, ctx: &mut ScriptContext) {
        ctx.world.destroy_structure(self.id);
    }

    fn move_to(&self, ctx: &mut ScriptContext, new_pos: &V3) {
        ctx.world.get_structure_mut(self.id)
           .map(|mut s| s.set_pos(*new_pos));
    }

    fn replace(&self, ctx: &mut ScriptContext, new_template_name: &str) {
        let new_template_id = match ctx.world.data().object_templates.find_id(new_template_name) {
            Some(x) => x,
            None => return,
        };

        ctx.world.get_structure_mut(self.id)
           .map(|mut s| s.set_template_id(new_template_id));
    }
}

impl_type_name!(Structure);
impl_metatable_key!(Structure);

impl Userdata for Structure {
    fn populate_table(lua: &mut LuaState) {
        methods_ctx!(lua, -1, Structure,
                     () => self.pos(),
                     () => self.size(),
                     () => self.delete(),
                     () => self.template(),
                     (pos) => self.move_to(pos),
                     (name) => self.replace(name));
    }
}


#[derive(Copy)]
struct Client {
    id: ClientId,
}

impl Client {
    fn world(&self) -> World {
        World
    }

    fn id(&self) -> i32 {
        self.id as i32
    }

    fn pawn(&self, ctx: &ScriptContext) -> Option<Entity> {
        ctx.world.get_client(self.id)
           .and_then(|c| c.pawn_id())
           .map(|eid| Entity { id: eid })
    }
}

impl_type_name!(Client);
impl_metatable_key!(Client);

impl Userdata for Client {
    fn populate_table(lua: &mut LuaState) {
        methods!(lua, -1, Client,
                 () => self.world(),
                 () => self.id());

        methods_ctx!(lua, -1, Client,
                     () => self.pawn());
    }
}


#[derive(Copy)]
struct Entity {
    id: EntityId,
}

impl Entity {
    fn world(&self) -> World {
        World
    }

    fn id(&self) -> i32 {
        self.id as i32
    }

    fn pos(&self, ctx: &ScriptContext) -> Option<V3> {
        ctx.world.get_entity(self.id).map(|e| e.pos(ctx.now))
    }

    fn facing(&self, ctx: &ScriptContext) -> Option<V3> {
        ctx.world.get_entity(self.id).map(|e| e.facing())
    }
}

impl_type_name!(Entity);
impl_metatable_key!(Entity);

impl Userdata for Entity {
    fn populate_table(lua: &mut LuaState) {
        methods!(lua, -1, Entity,
                 () => self.world(),
                 () => self.id());

        methods_ctx!(lua, -1, Entity,
                     () => self.pos(),
                     () => self.facing());
    }
}
