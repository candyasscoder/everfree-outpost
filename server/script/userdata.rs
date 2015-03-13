use std::borrow::ToOwned;
use libc::c_int;

use physics::{TILE_SIZE, CHUNK_SIZE};

use types::*;
use util::StrResult;
use util::Stable;

use engine::{Engine, logic};
use engine::glue::WorldFragment;
use lua::LuaState;
use world;
use world::Fragment;
use world::object::*;

use super::build_type_table;
use super::traits::Userdata;
use super::traits::{check_args, FromLua, ToLua};
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

mk_build_types_table!(V3, World,
                      Client, Entity, Structure, Inventory,
                      StableClient, StableEntity, StableStructure, StableInventory);


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
        fn $name(mut lua: LuaState) -> c_int {
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
        fn $name(mut lua: LuaState) -> c_int {
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
        fn $name(mut lua: LuaState) -> c_int {
            let (result, count): ($ret_ty, ::libc::c_int) = {
                let (($($arg,)*), count): (($($arg_ty,)*), ::libc::c_int) = unsafe {
                    $crate::script::traits::unpack_args_count(&mut lua, stringify!($name))
                };
                ($body, count)
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


impl_type_name!(V3);
impl_metatable_key!(V3);

impl Userdata for V3 {
    fn populate_table(lua: &mut LuaState) {
        lua_table_fns2! {
            lua, -1,

            fn x(ud: V3) -> i32 { ud.x }
            fn y(ud: V3) -> i32 { ud.y }
            fn z(ud: V3) -> i32 { ud.z }

            fn new(x: i32, y: i32, z: i32) -> V3 { V3::new(x, y, z) }

            fn abs(ud: V3) -> V3 { ud.abs() }
            fn extract(ud: V3) -> (i32, i32, i32) { (ud.x, ud.y, ud.z) }

            fn max(v: V3) -> i32 { v.max() }

            fn pixel_to_tile(ud: V3) -> V3 {
                ud.div_floor(scalar(TILE_SIZE))
            }

            fn tile_to_chunk(ud: V3) -> V3 {
                ud.div_floor(scalar(CHUNK_SIZE))
            }
        }
    }

    fn populate_metatable(lua: &mut LuaState) {
        lua_table_fns2! {
            lua, -1,

            fn __add(a: V3, b: V3) -> V3 { a + b }
            fn __sub(a: V3, b: V3) -> V3 { a - b }
            fn __mul(a: V3, b: V3) -> V3 { a * b }
            fn __div(a: V3, b: V3) -> V3 { a / b }
            fn __mod(a: V3, b: V3) -> V3 { a % b }
        }
    }
}


#[derive(Copy)]
pub struct World;

impl_type_name!(World);
impl_metatable_key!(World);

impl Userdata for World {
    fn populate_table(lua: &mut LuaState) {
        lua_table_fns2! {
            lua, -1,

            fn get() -> World {
                World
            }

            fn create_entity(!partial wf: WorldFragment,
                             pos: V3,
                             anim: AnimId,
                             appearance: u32) -> StrResult<Entity> {
                wf.create_entity(pos, anim, appearance)
                  .map(|e| Entity { id: e.id() })
            }

            fn find_structure_at_point(!partial w: &world::World,
                                       _w: World,
                                       pos: V3) -> Option<Structure> {
                let chunk = pos.reduce().div_floor(scalar(CHUNK_SIZE));
                for s in w.chunk_structures(chunk) {
                    if s.bounds().contains(pos) {
                        return Some(Structure { id: s.id() });
                    }
                };
                None
            }

            fn create_structure(!partial wf: WorldFragment,
                                _w: World,
                                pos: V3,
                                template_name: &str) -> StrResult<Structure> {{
                let template_id =
                    unwrap!(wf.data().object_templates.find_id(template_name),
                            "named structure template does not exist");

                wf.create_structure(pos, template_id)
                  .map(|s| Structure { id: s.id() })
            }}

            fn create_inventory(!partial wf: WorldFragment, _w: World) -> StrResult<Inventory> {
                wf.create_inventory()
                  .map(|i| Inventory { id: i.id() })
            }

            fn item_id_to_name(!partial w: &world::World, _w: World, id: ItemId) -> _ {
                w.data().item_data.get_name(id).map(|s| s.to_owned())
            }

            fn get_client(!partial w: &world::World,
                          _w: World,
                          id: ClientId) -> Option<Client> {
                w.get_client(id).map(|_| Client { id: id })
            }

            fn get_entity(!partial w: &world::World,
                          _w: World,
                          id: EntityId) -> Option<Entity> {
                w.get_entity(id).map(|_| Entity { id: id })
            }

            fn get_structure(!partial w: &world::World,
                             _w: World,
                             id: StructureId) -> Option<Structure> {
                w.get_structure(id).map(|_| Structure { id: id })
            }

            fn get_inventory(!partial w: &world::World,
                             _w: World,
                             id: InventoryId) -> Option<Inventory> {
                w.get_inventory(id).map(|_| Inventory { id: id })
            }
        }
    }
}


#[derive(Copy)]
pub struct Client {
    pub id: ClientId,
}

impl_type_name!(Client);
impl_metatable_key!(Client);

impl Userdata for Client {
    fn populate_table(lua: &mut LuaState) {
        lua_table_fns2! {
            lua, -1,

            fn world(_c: Client) -> World { World }
            fn id(c: Client) -> u16 { c.id.unwrap() }

            fn stable_id(!partial wf: WorldFragment, c: Client) -> Option<StableClient> {
                wf.get_client_mut(c.id)
                  .map(|mut c| StableClient { id: c.stable_id() })
            }

            fn name(!partial w: &world::World, c: Client) -> Option<String> {
                w.get_client(c.id)
                 .map(|c| c.name().to_owned())
            }

            fn pawn(!partial w: &world::World, c: Client) -> Option<Entity> {
                w.get_client(c.id)
                 .and_then(|c| c.pawn_id())
                 .map(|eid| Entity { id: eid })
            }

            fn set_pawn(!partial wf: WorldFragment, c: Client, e: Entity) -> StrResult<()> {
                let mut c = unwrap!(wf.get_client_mut(c.id));
                try!(c.set_pawn(Some(e.id)));
                Ok(())
            }

            fn clear_pawn(!partial wf: WorldFragment, c: Client) -> StrResult<()> {
                let mut c = unwrap!(wf.get_client_mut(c.id));
                try!(c.set_pawn(None));
                Ok(())
            }

            fn open_inventory(!full eng: &mut Engine,
                              c: Client,
                              i: Inventory) -> StrResult<()> {
                logic::open_inventory(eng, c.id, i.id)
            }

            fn open_container(!partial w: &world::World,
                              c: Client,
                              i1: Inventory,
                              i2: Inventory) -> StrResult<()> {
                // Check inputs are valid.
                unwrap!(w.get_client(c.id));
                unwrap!(w.get_inventory(i1.id));
                unwrap!(w.get_inventory(i2.id));

                //ctx.world.record(Update::ClientOpenContainer(c.id, i1.id, i2.id));
                Ok(())
            }

            fn open_crafting(!partial w: &world::World,
                             c: Client,
                             s: Structure,
                             i: Inventory) -> StrResult<()> {
                // Check inputs are valid.
                unwrap!(w.get_client(c.id));
                unwrap!(w.get_structure(s.id));
                unwrap!(w.get_inventory(i.id));

                //ctx.world.record(Update::ClientOpenCrafting(c.id, s.id, i.id));
                Ok(())
            }

            fn send_message(!partial w: &world::World,
                            c: Client,
                            msg: &str) -> StrResult<()> {
                unwrap!(w.get_client(c.id));
                //ctx.world.record(Update::ClientMessage(c.id, msg.to_owned()));
                Ok(())
            }
        }
    }
}



#[derive(Copy)]
pub struct Entity {
    pub id: EntityId,
}

impl_type_name!(Entity);
impl_metatable_key!(Entity);

impl Userdata for Entity {
    fn populate_table(lua: &mut LuaState) {
        use world::EntityAttachment;

        lua_table_fns2! {
            lua, -1,

            fn world(_e: Entity) -> World { World }
            fn id(e: Entity) -> u32 { e.id.unwrap() }

            fn stable_id(!partial wf: WorldFragment,
                         e: Entity) -> Option<StableEntity> {
                wf.get_entity_mut(e.id)
                  .map(|mut e| StableEntity { id: e.stable_id() })
            }

            fn destroy(!partial wf: WorldFragment,
                       e: Entity) -> StrResult<()> {
                wf.destroy_entity(e.id)
            }

            fn pos(!partial wf: WorldFragment, e: Entity) -> Option<V3> {
                let now = wf.now();
                wf.world().get_entity(e.id).map(|e| e.pos(now))
            }

            fn facing(!partial w: &world::World, e: Entity) -> Option<V3> {
                w.get_entity(e.id).map(|e| e.facing())
            }

            fn teleport(!partial wf: WorldFragment,
                        e: Entity,
                        pos: V3) -> StrResult<()> {
                let now = wf.now();
                let mut e = unwrap!(wf.get_entity_mut(e.id));
                e.set_motion(world::Motion::stationary(pos, now));
                Ok(())
            }

            // TODO: come up with a lua representation of attachment so we can unify these methods
            // and also return the previous attachment (like the underlying op does)
            fn attach_to_world(!partial wf: WorldFragment,
                               e: Entity) -> StrResult<()> {
                let mut e = unwrap!(wf.get_entity_mut(e.id));
                try!(e.set_attachment(EntityAttachment::World));
                Ok(())
            }

            fn attach_to_client(!partial wf: WorldFragment,
                                e: Entity,
                                c: Client) -> StrResult<()> {
                let mut e = unwrap!(wf.get_entity_mut(e.id));
                try!(e.set_attachment(EntityAttachment::Client(c.id)));
                Ok(())
            }
        }
    }
}


#[derive(Copy)]
pub struct Structure {
    pub id: StructureId,
}

impl_type_name!(Structure);
impl_metatable_key!(Structure);

impl Userdata for Structure {
    fn populate_table(lua: &mut LuaState) {
        use world::StructureAttachment;

        lua_table_fns2! {
            lua, -1,

            fn world(_s: Structure) -> World { World }
            fn id(s: Structure) -> u32 { s.id.unwrap() }

            fn stable_id(!partial wf: WorldFragment, s: Structure) -> Option<StableStructure> {
                wf.get_structure_mut(s.id)
                  .map(|mut s| StableStructure { id: s.stable_id() })
            }

            fn destroy(!partial wf: WorldFragment, s: Structure) -> StrResult<()> {
                wf.destroy_structure(s.id)
            }

            fn pos(!partial w: &world::World, s: Structure) -> Option<V3> {
                w.get_structure(s.id)
                 .map(|s| s.pos())
            }

            fn size(!partial w: &world::World, s: Structure) -> Option<V3> {
                w.get_structure(s.id)
                 .map(|s| s.size())
            }

            fn template_id(!partial w: &world::World, s: Structure) -> Option<u32> {
                w.get_structure(s.id)
                 .map(|s| s.template_id())
            }

            fn template(!partial w: &world::World, s: Structure) -> Option<String> {
                w.get_structure(s.id)
                 .map(|s| s.template_id())
                 .and_then(|id| w.data().object_templates.get_template(id))
                 .map(|t| t.name.clone())
            }

            fn move_to(!partial wf: WorldFragment, s: Structure, new_pos: V3) -> StrResult<()> {
                let mut s = unwrap!(wf.get_structure_mut(s.id));
                s.set_pos(new_pos)
            }

            fn replace(!partial wf: WorldFragment,
                       s: Structure,
                       new_template_name: &str) -> StrResult<()> {
                let new_template_id =
                    unwrap!(wf.data().object_templates.find_id(new_template_name),
                            "named structure template does not exist");

                let mut s = unwrap!(wf.get_structure_mut(s.id));
                s.set_template_id(new_template_id)
            }

            fn attach_to_world(!partial wf: WorldFragment, s: Structure) -> StrResult<()> {
                let mut s = unwrap!(wf.get_structure_mut(s.id));
                try!(s.set_attachment(StructureAttachment::World));
                Ok(())
            }

            fn attach_to_chunk(!partial wf: WorldFragment, s: Structure) -> StrResult<()> {
                let mut s = unwrap!(wf.get_structure_mut(s.id));
                try!(s.set_attachment(StructureAttachment::Chunk));
                Ok(())
            }
        }
    }
}


#[derive(Copy)]
pub struct Inventory {
    pub id: InventoryId,
}

impl_type_name!(Inventory);
impl_metatable_key!(Inventory);

impl Userdata for Inventory {
    fn populate_table(lua: &mut LuaState) {
        use world::InventoryAttachment;

        lua_table_fns2! {
            lua, -1,

            fn world(_i: Inventory) -> World { World }
            fn id(i: Inventory) -> u32 { i.id.unwrap() }

            fn stable_id(!partial wf: WorldFragment, i: Inventory) -> Option<StableInventory> {
                wf.get_inventory_mut(i.id)
                  .map(|mut i| StableInventory { id: i.stable_id() })
            }

            fn destroy(!partial wf: WorldFragment, i: Inventory) -> StrResult<()> {
                wf.destroy_inventory(i.id)
            }

            fn count(!partial w: &world::World, i: Inventory, name: &str) -> StrResult<u8> {
                let i = unwrap!(w.get_inventory(i.id));
                i.count_by_name(name)
            }

            fn update(!partial wf: WorldFragment,
                      i: Inventory,
                      name: &str,
                      adjust: i16) -> StrResult<u8> {
                let mut i = unwrap!(wf.get_inventory_mut(i.id));
                i.update_by_name(name, adjust)
            }

            fn attach_to_world(!partial wf: WorldFragment,
                               i: Inventory) -> StrResult<()> {
                let mut i = unwrap!(wf.get_inventory_mut(i.id));
                try!(i.set_attachment(InventoryAttachment::World));
                Ok(())
            }

            fn attach_to_client(!partial wf: WorldFragment,
                                i: Inventory,
                                c: Client) -> StrResult<()> {
                let mut i = unwrap!(wf.get_inventory_mut(i.id));
                try!(i.set_attachment(InventoryAttachment::Client(c.id)));
                Ok(())
            }

            fn attach_to_entity(!partial wf: WorldFragment,
                                i: Inventory,
                                e: Entity) -> StrResult<()> {
                let mut i = unwrap!(wf.get_inventory_mut(i.id));
                try!(i.set_attachment(InventoryAttachment::Entity(e.id)));
                Ok(())
            }

            fn attach_to_structure(!partial wf: WorldFragment,
                                   i: Inventory,
                                   s: Structure) -> StrResult<()> {
                let mut i = unwrap!(wf.get_inventory_mut(i.id));
                try!(i.set_attachment(InventoryAttachment::Structure(s.id)));
                Ok(())
            }
        }
    }
}


macro_rules! define_stable_wrapper {
    ($name:ident, $obj_ty:ident, $id_ty:ty, $transient_id:ident) => {
        #[derive(Copy)]
        pub struct $name {
            pub id: Stable<$id_ty>,
        }

        impl_type_name!($name);
        impl_metatable_key!($name);

        impl Userdata for $name {
            fn populate_table(lua: &mut LuaState) {
                lua_table_fns2! {
                    lua, -1,

                    fn id(stable: $name) -> String {
                        format!("{:x}", stable.id.val)
                    }

                    fn get(!partial w: &world::World, stable: $name) -> Option<$obj_ty> {
                        w.$transient_id(stable.id)
                         .map(|id| $obj_ty { id: id })
                    }
                }
            }

            fn populate_metatable(lua: &mut LuaState) {
                lua_table_fns2! {
                    lua, -1,

                    fn __eq(a: $name, b: $name) -> bool {
                        a.id == b.id
                    }
                }
            }
        }
    };
}

define_stable_wrapper!(StableClient, Client, ClientId, transient_client_id);
define_stable_wrapper!(StableEntity, Entity, EntityId, transient_entity_id);
define_stable_wrapper!(StableStructure, Structure, StructureId, transient_structure_id);
define_stable_wrapper!(StableInventory, Inventory, InventoryId, transient_inventory_id);
