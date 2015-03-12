use std::borrow::ToOwned;
use std::cell::{RefCell, RefMut};
use std::mem;
use libc::c_int;

use types::*;

use data::Data;
use engine;
use input::ActionId;
use world;
use world::object::*;

use lua::{OwnedLuaState, LuaState};
use lua::{GLOBALS_INDEX, REGISTRY_INDEX};

use self::traits::pack_count;
use self::traits::Userdata;
use self::traits::ToLua;
use self::traits::{MetatableKey, metatable_key};
pub use self::save::{WriteHooks, ReadHooks};


#[macro_use] mod traits;
mod userdata;
mod save;


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

#[derive(Copy, PartialEq, Eq, Hash, Debug)]
pub struct Nil;

pub struct ScriptEngine {
    owned_lua: OwnedLuaState,
}

struct ScriptContext<'a, 'd: 'a> {
    engine: &'a mut engine::Engine<'d>,
    now: Time,
}

impl<'a, 'd> ScriptContext<'a, 'd> {
    pub fn new(engine: &'a mut engine::Engine<'d>, now: Time) -> ScriptContext<'a, 'd> {
        ScriptContext {
            engine: engine,
            now: now,
        }
    }

    pub fn data(&self) -> &'d Data {
        self.world().data()
    }

    pub fn world(&self) -> &world::World<'d> {
        &self.engine.world
    }

    pub fn world_mut(&mut self) -> &mut world::World<'d> {
        &mut self.engine.world
    }

    pub fn world_frag<'b>(&'b mut self) -> engine::glue::WorldFragment<'b, 'd> {
        engine::split::EngineRef::new(&mut self.engine).slice()
    }
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
        run_callback(&mut self.owned_lua.get(),
                     "outpost_callback_set_client_extra",
                     (cid.unwrap(), Nil));
    }

    pub fn callback_entity_destroyed(&mut self, eid: EntityId) {
        run_callback(&mut self.owned_lua.get(),
                     "outpost_callback_set_entity_extra",
                     (eid.unwrap(), Nil));
    }

    pub fn callback_structure_destroyed(&mut self, sid: StructureId) {
        run_callback(&mut self.owned_lua.get(),
                     "outpost_callback_set_structure_extra",
                     (sid.unwrap(), Nil));
    }

    pub fn callback_inventory_destroyed(&mut self, iid: InventoryId) {
        run_callback(&mut self.owned_lua.get(),
                     "outpost_callback_set_inventory_extra",
                     (iid.unwrap(), Nil));
    }
}

pub unsafe fn get_ctx_ref<'a>(lua: &mut LuaState) -> &'a RefCell<ScriptContext<'a, 'a>> {
    lua.get_field(REGISTRY_INDEX, CTX_KEY);
    let raw_ptr: *mut RefCell<ScriptContext> = lua.to_userdata_raw(-1);
    lua.pop(1);

    if raw_ptr.is_null() {
        panic!("tried to access script context, but no context is active");
    }

    mem::transmute(raw_ptr)
}

pub unsafe fn get_ctx<'a>(lua: &mut LuaState) -> RefMut<'a, ScriptContext<'a, 'a>> {
    get_ctx_ref(lua).borrow_mut()
}

pub fn with_ctx<T, F: FnOnce(&mut ScriptContext) -> T>(lua: &mut LuaState, f: F) -> T {
    let mut ctx = unsafe { get_ctx(lua) };
    f(&mut *ctx)
}

fn run_callback<A: ToLua>(lua: &mut LuaState, key: &str, args: A) {
    lua.get_field(REGISTRY_INDEX, key);
    let arg_count = pack_count(lua, args);
    lua.pcall(arg_count, 0, 0).unwrap();
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




