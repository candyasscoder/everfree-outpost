use std::cell::RefCell;

use util::StrResult;

use lua::LuaState;
use script::build_type_table;
use script::traits::TypeName;


macro_rules! insert_function {
    ($lua:expr, $idx:expr, $name:expr, $func:expr) => {{
        $lua.push_rust_function($func);
        $lua.set_field($idx - 1, $name);
    }}
}

/// Helper macro for parsing a block out of a function body.  '-> $t:ty $b:block' is prohibited (ty
/// may not be followed by block), so instead, match '-> $t:ty { $($b:tt)* }' and then invoke
/// 'mk_block!({ $($b)* } {})' to produce the actual block.
macro_rules! mk_block {
  ({ $s:stmt; $($t:tt)* } { $($ss:stmt;)* }) => { mk_block!({ $($t)* } {$($ss;)* $s;}) };
  ({ $e:expr } { $($ss:stmt;)* }) => {{ $($ss;)* $e }};
  ({} { $($ss:stmt;)* }) => {{ $($ss;)* }};
}

macro_rules! lua_fn_raw {
    // TODO: support functions that take only the context and no other args
    ($name:ident,
     (!partial $ctx:ident: $ctx_ty:ty, $($arg:ident: $arg_ty:ty),*),
     $ret_ty:ty,
     $body:expr) => {
        #[allow(unused_mut)]
        fn $name(mut lua: $crate::lua::LuaState) -> ::libc::c_int {
            let (result, count): ($ret_ty, ::libc::c_int) = {
                let ctx = unsafe { $crate::script::PartialContext::from_lua(&mut lua) };
                let (args, count): (_, ::libc::c_int) = unsafe {
                    $crate::script::traits::unpack_args_count(&mut lua, stringify!($name))
                };
                // Use a closure to prevent $body from abusing the context reference, which will
                // likely be inferred as 'static.
                let f = |mut $ctx: $ctx_ty, ($($arg,)*): ($($arg_ty,)*)| $body;
                (f(ctx, args), count)
            };
            lua.pop(count);
            $crate::script::traits::pack_count(&mut lua, result)
        }
    };

    ($name:ident,
     (!full $ctx:ident: $ctx_ty:ty, $($arg:ident: $arg_ty:ty),*),
     $ret_ty:ty,
     $body:expr) => {
        #[allow(unused_mut)]
        fn $name(mut lua: $crate::lua::LuaState) -> ::libc::c_int {
            let result: $ret_ty = {
                unsafe { <$ctx_ty as $crate::script::FullContext>::check(&mut lua) };
                let (args, count): (_, ::libc::c_int) = unsafe {
                    $crate::script::traits::unpack_args_count(&mut lua, stringify!($name))
                };
                // Clear the stack in case of reentrant calls to the script engine.
                lua.pop(count);
                let ctx = unsafe { $crate::script::FullContext::from_lua(&mut lua) };
                let f = |mut $ctx: $ctx_ty, ($($arg,)*): ($($arg_ty,)*)| $body;
                f(ctx, args)
            };
            $crate::script::traits::pack_count(&mut lua, result)
        }
    };

    ($name:ident,
     ($($arg:ident: $arg_ty:ty),*),
     $ret_ty:ty,
     $body:expr) => {
        #[allow(unused_mut)]
        fn $name(mut lua: $crate::lua::LuaState) -> ::libc::c_int {
            let (result, count): ($ret_ty, ::libc::c_int) = {
                let (args, count): (_, ::libc::c_int) = unsafe {
                    $crate::script::traits::unpack_args_count(&mut lua, stringify!($name))
                };
                let f = |($($arg,)*): ($($arg_ty,)*)| $body;
                (f(args), count)
            };
            lua.pop(count);
            $crate::script::traits::pack_count(&mut lua, result)
        }
    };
}

macro_rules! lua_table_fns2 {
    ( $lua:expr, $idx: expr,
        $(
            fn $name:ident( $($a:tt)* ) -> $ret_ty:ty { $($b:tt)* }
                //$(! $mode:ident)*
                //$($arg_name:ident : $arg_ty:ty),*
        )*
    ) => {{
        $(
            lua_fn_raw!($name, ( $($a)* ), $ret_ty, mk_block!({ $($b)* } {}));
            insert_function!($lua, $idx, stringify!($name), $name);
        )*
    }};
}

pub mod terrain_gen;
pub mod types;
pub mod world;

macro_rules! mk_build_types_table {
    ($($ty:ty,)*) => {
        pub fn build_types_table(lua: &mut LuaState) {
            lua.push_table();
            $({
                build_type_table::<$ty>(lua);
                lua.set_field(-2, <$ty as TypeName>::type_name());
            })*
        }
    }
}

mk_build_types_table!(
    ::types::V3,
    ::types::V2,

    self::world::World,
    self::world::Client,
    self::world::Entity,
    self::world::Structure,
    self::world::Inventory,
    self::world::StableClient,
    self::world::StableEntity,
    self::world::StableStructure,
    self::world::StableInventory,

    self::terrain_gen::Rng,
    self::terrain_gen::GenChunk,

    self::terrain_gen::Values,
    self::terrain_gen::ValuesMut,
    self::terrain_gen::Points,

    self::terrain_gen::Field,
    self::terrain_gen::ConstantField,
    self::terrain_gen::RandomField,
    self::terrain_gen::DiamondSquare,

    self::terrain_gen::IsoDiskSampler,
);


pub struct Wrapper<T> {
    pub x: RefCell<T>,
}

impl<T> Wrapper<T> {
    pub fn new(x: T) -> Wrapper<T> {
        Wrapper {
            x: RefCell::new(x),
        }
    }

    pub fn open<F, R>(&self, f: F) -> R
            where F: FnOnce(&mut T) -> R {
        let mut b = self.x.borrow_mut();
        f(&mut *b)
    }
}


pub struct OptWrapper<T> {
    pub x: RefCell<Option<T>>,
}

impl<T> OptWrapper<T> {
    pub fn new(x: T) -> OptWrapper<T> {
        OptWrapper {
            x: RefCell::new(Some(x)),
        }
    }

    pub fn take(&self) -> Option<T> {
        self.x.borrow_mut().take()
    }

    pub fn open<F, R>(&self, f: F) -> StrResult<R>
            where F: FnOnce(&mut T) -> R {
        let mut b = self.x.borrow_mut();
        let x = unwrap!(b.as_mut());
        Ok(f(x))
    }
}
