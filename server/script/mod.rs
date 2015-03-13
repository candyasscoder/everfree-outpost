use std::borrow::ToOwned;
use std::cell::{RefCell, RefMut};
use std::error;
use std::marker::MarkerTrait;
use std::mem;
use libc::c_int;

use types::*;
use util::{StringError, StringResult};

use data::Data;
use engine;
use engine::glue::WorldFragment;
use input::ActionId;
use world;
use world::object::*;

use lua::{OwnedLuaState, LuaState};
use lua::{GLOBALS_INDEX, REGISTRY_INDEX};

pub use self::save::{WriteHooks, ReadHooks};
use self::traits::pack_count;
use self::traits::Userdata;
use self::traits::ToLua;
use self::traits::{MetatableKey, metatable_key};


#[macro_use] mod traits;
mod userdata;
mod save;


const FFI_CALLBACKS_KEY: &'static str = "outpost_ffi_callbacks";
const FFI_LIB_NAME: &'static str = "outpost_ffi";

const BOOTSTRAP_FILE: &'static str = "bootstrap.lua";


#[derive(Copy, PartialEq, Eq, Hash, Debug)]
pub struct Nil;

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

            // Put an empty table into _LOADED.bootstrap, to prevent "require('bootstrap')" from
            // causing problems.
            lua.get_field(REGISTRY_INDEX, "_LOADED");
            lua.push_string("bootstrap");
            lua.push_table();
            lua.set_table(-3);
            lua.pop(1);

            // Set package.path to the script directory.
            lua.get_field(GLOBALS_INDEX, "package");
            lua.push_string("path");
            lua.push_string(&*format!("{}/?.lua", script_dir.as_str().unwrap()));
            lua.set_table(-3);
            lua.pop(1);

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


            // Finally, actually run the startup script.
            lua.load_file(&script_dir.join(BOOTSTRAP_FILE)).unwrap();
            lua.pcall(0, 0, 0).unwrap();
        }

        ScriptEngine {
            owned_lua: owned_lua,
        }
    }

    fn with_context<F, R, C>(&mut self,
                             ptr: *mut C,
                             f: F) -> R
            where F: FnOnce(&mut LuaState) -> R,
                  C: BaseContext {

        let mut lua = self.owned_lua.get();

        let old_ptr: *mut C = unsafe { get_ctx_raw(&mut lua) };
        unsafe { set_ctx(&mut lua, ptr) };

        let x = f(&mut lua);

        unsafe { set_ctx(&mut lua, old_ptr) };

        x
    }

    pub fn cb_open_inventory(eng: &mut engine::Engine, cid: ClientId) -> StringResult<()> {
        let ptr = eng as *mut engine::Engine;
        eng.script.with_context(ptr, |lua| {
            run_callback(lua,
                         "outpost_callback_open_inventory",
                         (userdata::Client { id: cid }))
        })
    }

    /*
    pub fn eval(&mut self,
                world: &mut world::World,
                now: Time,
                code: &str) -> Result<String, String> {
        let ctx = RefCell::new(ScriptContext {
            world: world,
            now: now,
        });
        self.with_context(&ctx, |lua| {
            lua.get_field(REGISTRY_INDEX, "outpost_callback_eval");
            userdata::World.to_lua(lua);
            code.to_lua(lua);
            try!(lua.pcall(2, 1, 0).map_err(|(e,s)| format!("{:?}: {}", e, s)));

            let result = lua.to_string(-1).unwrap_or("(bad result)");
            Ok(result.to_owned())

        })
    }
    */

    /*
    pub fn callback_action(&mut self,
                           world: &mut world::World,
                           now: Time,
                           id: ClientId,
                           action: ActionId,
                           arg: u32) -> Result<(), String> {
        let ctx = RefCell::new(ScriptContext {
            world: world,
            now: now,
        });
        self.with_context(&ctx, |lua| {
            lua.get_field(REGISTRY_INDEX, "outpost_callback_action");
            let c = userdata::Client { id: id };
            c.to_lua(lua);
            action.to_lua(lua);
            arg.to_lua(lua);
            lua.pcall(3, 0, 0).map_err(|(e,s)| format!("{:?}: {}", e, s))
        })
    }

    pub fn callback_command(&mut self,
                            world: &mut world::World,
                            now: Time,
                            id: ClientId,
                            msg: &str) -> Result<(), String> {
        let ctx = RefCell::new(ScriptContext {
            world: world,
            now: now,
        });
        self.with_context(&ctx, |lua| {
            lua.get_field(REGISTRY_INDEX, "outpost_callback_command");
            let c = userdata::Client { id: id };
            c.to_lua(lua);
            msg.to_lua(lua);
            lua.pcall(2, 0, 0).map_err(|(e,s)| format!("{:?}: {}", e, s))
        })
    }
    */

    pub fn callback_client_destroyed(&mut self, cid: ClientId) {
        warn_on_err!(run_callback(&mut self.owned_lua.get(),
                                  "outpost_callback_set_client_extra",
                                  (cid.unwrap(), Nil)));
    }

    pub fn callback_entity_destroyed(&mut self, eid: EntityId) {
        warn_on_err!(run_callback(&mut self.owned_lua.get(),
                                  "outpost_callback_set_entity_extra",
                                  (eid.unwrap(), Nil)));
    }

    pub fn callback_structure_destroyed(&mut self, sid: StructureId) {
        warn_on_err!(run_callback(&mut self.owned_lua.get(),
                                  "outpost_callback_set_structure_extra",
                                  (sid.unwrap(), Nil)));
    }

    pub fn callback_inventory_destroyed(&mut self, iid: InventoryId) {
        warn_on_err!(run_callback(&mut self.owned_lua.get(),
                                  "outpost_callback_set_inventory_extra",
                                  (iid.unwrap(), Nil)));
    }
}


fn run_callback<A: ToLua>(lua: &mut LuaState, key: &str, args: A) -> StringResult<()> {
    lua.get_field(REGISTRY_INDEX, key);
    let arg_count = pack_count(lua, args);
    lua.pcall(arg_count, 0, 0)
       .map_err(|(e, s)| StringError { msg: format!("{:?}: {}", e, s) })
}

fn build_ffi_lib(lua: &mut LuaState) {
    lua.push_table();

    userdata::build_types_table(lua);
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

fn build_callbacks_table(lua: &mut LuaState) {
    lua.push_table();

    lua.push_table();
    lua.push_rust_function(callbacks_table_newindex);
    lua.set_field(-2, "__newindex");
    lua.push_rust_function(callbacks_table_index);
    lua.set_field(-2, "__index");
    lua.set_metatable(-2);
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


impl ToLua for ActionId {
    fn to_lua(self, lua: &mut LuaState) {
        use input::*;
        let name = match self {
            ACTION_USE => "use",
            ACTION_INVENTORY => "inventory",
            ACTION_USE_ITEM => "use_item",
            ActionId(id) => {
                lua.push_string(&*format!("unknown_{}", id));
                return;
            },
        };
        lua.push_string(name);
    }
}


trait BaseContext: MarkerTrait {
    fn registry_key() -> &'static str;
}

unsafe fn get_ctx_raw<C: BaseContext>(lua: &mut LuaState) -> *mut C {
    lua.get_field(REGISTRY_INDEX, C::registry_key());
    let ptr = lua.to_userdata_raw(-1);
    lua.pop(1);

    ptr
}

unsafe fn get_ctx<C: BaseContext>(lua: &mut LuaState) -> *mut C {
    let ptr = get_ctx_raw::<C>(lua);

    if ptr.is_null() {
        lua.push_string(&*format!("required context {:?} is not available",
                                  C::registry_key()));
        lua.error();
    }

    ptr
}

unsafe fn set_ctx<C: BaseContext>(lua: &mut LuaState, ptr: *mut C) {
    lua.push_light_userdata(ptr);
    lua.set_field(REGISTRY_INDEX, C::registry_key());
}

unsafe trait FullContext<'a> {
    unsafe fn check(lua: &'a mut LuaState);
    unsafe fn from_lua(lua: &'a mut LuaState) -> Self;
}

unsafe trait PartialContext {
    unsafe fn from_lua(lua: &mut LuaState) -> Self;
}


impl<'d> BaseContext for engine::Engine<'d> {
    fn registry_key() -> &'static str { "outpost_engine" }
}

unsafe impl<'a, 'd: 'a> FullContext<'a> for &'a mut engine::Engine<'d> {
    unsafe fn check(lua: &mut LuaState) {
        // Run get_ctx to check that the context is available.
        get_ctx::<engine::Engine>(lua);
    }

    unsafe fn from_lua(lua: &mut LuaState) -> &'a mut engine::Engine<'d> {
        let ptr = get_ctx_raw::<engine::Engine>(lua);
        assert!(!ptr.is_null());
        mem::transmute(ptr)
    }
}

unsafe impl<'a, 'd: 'a> PartialContext for WorldFragment<'a, 'd> {
    unsafe fn from_lua(lua: &mut LuaState) -> WorldFragment<'a, 'd> {
        let ptr = get_ctx::<engine::Engine>(lua);
        mem::transmute(ptr)
    }
}

unsafe impl<'a, 'd: 'a> PartialContext for &'a mut world::World<'d> {
    unsafe fn from_lua(lua: &mut LuaState) -> &'a mut world::World<'d> {
        let mut frag = WorldFragment::from_lua(lua);
        let ptr: &mut world::World = frag.world_mut();
        mem::transmute(ptr)
    }
}

unsafe impl<'a, 'd: 'a> PartialContext for &'a world::World<'d> {
    unsafe fn from_lua(lua: &mut LuaState) -> &'a world::World<'d> {
        let frag = WorldFragment::from_lua(lua);
        let ptr: &world::World = frag.world();
        mem::transmute(ptr)
    }
}
