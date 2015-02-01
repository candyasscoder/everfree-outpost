use std::ops::{Add, Sub, Mul, Div, Rem};
use libc::c_int;

use physics::{TILE_SIZE, CHUNK_SIZE};
use physics::v3::{Vn, V3, scalar};

use lua::LuaState;
use types::*;
use util::StrResult;
use world::object::*;

use super::{ScriptContext, get_ctx};
use super::build_type_table;
use super::traits::{Userdata, create_userdata};
use super::traits::{check_unpack, check_unpack_count, pack_count};
use super::traits::{TypeName, type_name};

macro_rules! mk_build_types_table {
    ($($ty:ty),*) => {
        pub fn build_types_table(lua: &mut LuaState) {
            lua.push_table();
            $({
                build_type_table::<$ty>(lua);
                lua.set_field(-2, <$ty as TypeName>::type_name());
            })*
        }
    }
}

mk_build_types_table!(V3, World, Structure, Client, Entity);



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
               }
               fn pixel_to_tile(p: &V3) { p.div_floor(scalar(TILE_SIZE)) }
               fn tile_to_chunk(p: &V3) { p.div_floor(scalar(CHUNK_SIZE)) }
        );
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

    fn create_structure(&self, ctx: &mut ScriptContext, pos: &V3, template_id: u32) -> StrResult<Structure> {
        ctx.world.create_structure(*pos, template_id)
           .map(|s| Structure { id: s.id() })
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
    fn world(&self) -> World {
        World
    }

    fn id(&self) -> i32 {
        self.id.unwrap() as i32
    }

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

    fn delete(&self, ctx: &mut ScriptContext) -> StrResult<()> {
        ctx.world.destroy_structure(self.id)
    }

    fn move_to(&self, ctx: &mut ScriptContext, new_pos: &V3) -> StrResult<()> {
        let mut s = unwrap!(ctx.world.get_structure_mut(self.id));
        s.set_pos(*new_pos)
    }

    fn replace(&self, ctx: &mut ScriptContext, new_template_name: &str) -> StrResult<()> {
        let new_template_id =
            unwrap!(ctx.world.data().object_templates.find_id(new_template_name),
                    "named structure template does not exist");

        let mut s = unwrap!(ctx.world.get_structure_mut(self.id));
        s.set_template_id(new_template_id)
    }
}

impl_type_name!(Structure);
impl_metatable_key!(Structure);

impl Userdata for Structure {
    fn populate_table(lua: &mut LuaState) {
        methods!(lua, -1, Structure,
                 () => self.world(),
                 () => self.id());

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
pub struct Client {
    pub id: ClientId,
}

impl Client {
    fn world(&self) -> World {
        World
    }

    fn id(&self) -> i32 {
        self.id.unwrap() as i32
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
        self.id.unwrap() as i32
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
