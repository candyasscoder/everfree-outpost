use std::mem;
use std::path::Path;
use std::ptr;
use libc::c_int;
use rand::XorShiftRng;

use types::*;
use util::{StringError, StringResult};

use engine;
use engine::glue::WorldFragment;
use engine::split::EngineRef;
use messages;
use msg;
use terrain_gen;
use timer;
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


#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
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
            lua.push_string(&*format!("{}/?.lua",
                                      script_dir.to_str().expect("invalid utf-8 in path")));
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

    fn with_context<F, R, E>(&mut self,
                             ptr: *mut E,
                             f: F) -> R
            where F: FnOnce(&mut LuaState) -> R,
                  E: engine::split::PartFlags {

        let mut lua = self.owned_lua.get();

        // `ptr` points to an engine part.  An engine part is a wrapper around `&mut Engine`.
        let data = (unsafe { ptr::read(ptr as *mut *mut engine::Engine) },
                    <E as engine::split::PartFlags>::flags());

        let old_ptr = unsafe { get_ctx_raw(&mut lua) };
        unsafe { set_ctx(&mut lua, &data) };

        // Clear the stack, then run the function.
        let count = lua.top_index();
        lua.pop(count);
        let x = f(&mut lua);

        unsafe { set_ctx(&mut lua, old_ptr) };

        x
    }

    fn with_engine<F, R>(eng: &mut engine::Engine,
                         f: F) -> R
            where F: FnOnce(&mut LuaState) -> R {
        let ptr: *mut EngineRef = unsafe { mem::transmute(&eng) };
        eng.script.with_context(ptr, f)
    }

    pub fn cb_chat_command(eng: &mut engine::Engine,
                           cid: ClientId,
                           msg: &str) -> StringResult<()> {
        ScriptEngine::with_engine(eng, |lua| {
            run_callback(lua,
                         "outpost_callback_command",
                         (userdata::world::Client { id: cid }, msg))
        })
    }

    pub fn cb_login(eng: &mut engine::Engine, cid: ClientId) -> StringResult<()> {
        ScriptEngine::with_engine(eng, |lua| {
            run_callback(lua,
                         "outpost_callback_login",
                         (userdata::world::Client { id: cid }))
        })
    }

    pub fn cb_open_inventory(eng: &mut engine::Engine, cid: ClientId) -> StringResult<()> {
        ScriptEngine::with_engine(eng, |lua| {
            run_callback(lua,
                         "outpost_callback_open_inventory",
                         (userdata::world::Client { id: cid }))
        })
    }


    pub fn cb_interact(eng: &mut engine::Engine,
                       cid: ClientId,
                       args: Option<msg::ExtraArg>) -> StringResult<()> {
        ScriptEngine::with_engine(eng, |lua| {
            run_callback(lua,
                         "outpost_callback_interact",
                         (userdata::world::Client { id: cid },
                          args.map(|a| userdata::extra_arg::ExtraArg::new(a))))
        })
    }

    pub fn cb_use_item(eng: &mut engine::Engine,
                       cid: ClientId,
                       item_id: ItemId,
                       args: Option<msg::ExtraArg>) -> StringResult<()> {
        ScriptEngine::with_engine(eng, |lua| {
            run_callback(lua,
                         "outpost_callback_use_item",
                         (userdata::world::Client { id: cid },
                          item_id,
                          args.map(|a| userdata::extra_arg::ExtraArg::new(a))))
        })
    }

    pub fn cb_use_ability(eng: &mut engine::Engine,
                          cid: ClientId,
                          item_id: ItemId,
                          args: Option<msg::ExtraArg>) -> StringResult<()> {
        ScriptEngine::with_engine(eng, |lua| {
            run_callback(lua,
                         "outpost_callback_use_ability",
                         (userdata::world::Client { id: cid },
                          item_id,
                          args.map(|a| userdata::extra_arg::ExtraArg::new(a))))
        })
    }


    pub fn cb_timeout(eng: &mut engine::Engine,
                      x: u32) -> StringResult<()> {
        ScriptEngine::with_engine(eng, |lua| {
            run_callback(lua,
                         "outpost_callback_timeout",
                         x)
        })
    }


    pub fn cb_eval(eng: &mut engine::Engine,
                   code: &str) -> Result<String, String> {
        ScriptEngine::with_engine(eng, |lua| {
            lua.get_field(REGISTRY_INDEX, "outpost_callback_eval");
            userdata::world::World.to_lua(lua);
            code.to_lua(lua);
            try!(lua.pcall(2, 1, 0).map_err(|(e,s)| format!("{:?}: {}", e, s)));
            let result = lua.to_string(-1).unwrap_or("(bad result)");
            Ok(result.to_owned())
        })
    }

    pub fn cb_client_destroyed(&mut self, cid: ClientId) {
        warn_on_err!(run_callback(&mut self.owned_lua.get(),
                                  "outpost_callback_set_client_extra",
                                  (cid.unwrap(), Nil)));
    }

    pub fn cb_entity_destroyed(&mut self, eid: EntityId) {
        warn_on_err!(run_callback(&mut self.owned_lua.get(),
                                  "outpost_callback_set_entity_extra",
                                  (eid.unwrap(), Nil)));
    }

    pub fn cb_structure_destroyed(&mut self, sid: StructureId) {
        warn_on_err!(run_callback(&mut self.owned_lua.get(),
                                  "outpost_callback_set_structure_extra",
                                  (sid.unwrap(), Nil)));
    }

    pub fn cb_inventory_destroyed(&mut self, iid: InventoryId) {
        warn_on_err!(run_callback(&mut self.owned_lua.get(),
                                  "outpost_callback_set_inventory_extra",
                                  (iid.unwrap(), Nil)));
    }

    pub fn cb_generate_chunk(tgf: &mut engine::glue::TerrainGenFragment,
                             plane_name: &str,
                             cpos: V2,
                             plane_rng: XorShiftRng,
                             chunk_rng: XorShiftRng) -> StringResult<terrain_gen::GenChunk> {
        use script::userdata::terrain_gen::GenChunk;
        let ptr: *mut _ = tgf;
        tgf.script_mut().with_context(ptr, |lua| {
            let gc = terrain_gen::GenChunk::new();
            let gc_wrap = userdata::terrain_gen::GenChunk::new(gc);
            gc_wrap.to_lua(lua);
            let gc_idx = lua.top_index();

            lua.get_field(REGISTRY_INDEX, "outpost_callback_generate_chunk");
            lua.copy(gc_idx);
            plane_name.to_lua(lua);
            cpos.to_lua(lua);
            userdata::terrain_gen::Rng::new(plane_rng).to_lua(lua);
            userdata::terrain_gen::Rng::new(chunk_rng).to_lua(lua);
            try!(lua.pcall(5, 0, 0)
                    .map_err(|(e, s)| StringError { msg: format!("{:?}: {}", e, s) }));

            let gc = {
                let gc_wrap = unwrap!(unsafe { lua.to_userdata::<GenChunk>(gc_idx) });
                unwrap!(gc_wrap.take())
            };
            lua.pop(1);
            Ok(gc)
        })
    }

    pub fn cb_apply_structure_extra<'d, WF>(wf: &mut WF,
                                            sid: StructureId,
                                            key: &str,
                                            value: &str) -> StringResult<()>
            where WF: world::Fragment<'d> {
        // FIXME: SUPER UNSAFE!!!
        // The correct solution is to add a PartFlags bound here and in terrain_gen::Fragment::WF,
        // but doing so causes incomprehensible lifetime errors in engine::glue...
        let wf: &mut engine::glue::WorldFragment = unsafe { mem::transmute(wf) };
        let ptr: *mut _ = wf;
        wf.script_mut().with_context(ptr, |lua| {
            run_callback(lua,
                         "outpost_callback_apply_structure_extra",
                         (userdata::world::Structure { id: sid }, key, value))
        })
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


unsafe fn get_ctx_raw(lua: &mut LuaState) -> *const (*mut engine::Engine<'static>, usize) {
    lua.get_field(REGISTRY_INDEX, "outpost_engine");
    let ptr = lua.to_userdata_raw(-1);
    lua.pop(1);

    ptr
}

unsafe fn get_ctx(lua: &mut LuaState) -> *const (*mut engine::Engine<'static>, usize) {
    let ptr = get_ctx_raw(lua);

    if ptr.is_null() {
        lua.push_string("required context outpost_engine is not available");
        lua.error();
    }

    ptr
}

unsafe fn set_ctx(lua: &mut LuaState, ptr: *const (*mut engine::Engine, usize)) {
    lua.push_light_userdata(ptr as *mut (*mut engine::Engine, usize));
    lua.set_field(REGISTRY_INDEX, "outpost_engine");
}

unsafe trait FullContext<'a> {
    unsafe fn check(lua: &'a mut LuaState);
    unsafe fn from_lua(lua: &'a mut LuaState) -> Self;
}

unsafe trait PartialContext {
    unsafe fn from_lua(lua: &mut LuaState) -> Self;
}

unsafe fn check_ctx_flags<E>(lua: &mut LuaState,
                             ptr: *const (*mut engine::Engine, usize))
        where E: engine::split::PartFlags {
    let need_flags = <E as engine::split::PartFlags>::flags();
    if (*ptr).1 & need_flags != need_flags {
        lua.push_string("outpost_engine context is missing flags");
        lua.error();
    }
}

unsafe fn cast_ctx<E>(ptr: *const (*mut engine::Engine, usize)) -> E
        where E: engine::split::PartFlags + engine::split::Part {
    let need_flags = <E as engine::split::PartFlags>::flags();
    assert!((*ptr).1 & need_flags == need_flags);
    <E as engine::split::Part>::from_ptr((*ptr).0)
}

unsafe fn get_cast_ctx<E>(lua: &mut LuaState) -> E
        where E: engine::split::PartFlags + engine::split::Part {
    let ptr = get_ctx(lua);
    check_ctx_flags::<E>(lua, ptr);
    <E as engine::split::Part>::from_ptr((*ptr).0)
}

unsafe impl<'a, 'd: 'a> FullContext<'a> for EngineRef<'a, 'd> {
    unsafe fn check(lua: &'a mut LuaState) {
        // Run get_ctx to check that the context is available.
        let ptr = get_ctx(lua);
        check_ctx_flags::<Self>(lua, ptr);
    }

    unsafe fn from_lua(lua: &'a mut LuaState) -> EngineRef<'a, 'd> {
        let ptr = get_ctx_raw(lua);
        assert!(!ptr.is_null());
        cast_ctx(ptr)
    }
}

unsafe impl<'a, 'd: 'a> FullContext<'a> for &'a mut engine::Engine<'d> {
    unsafe fn check(lua: &'a mut LuaState) {
        <EngineRef<'a, 'd> as FullContext>::check(lua)
    }

    unsafe fn from_lua(lua: &'a mut LuaState) -> &'a mut engine::Engine<'d> {
        <EngineRef<'a, 'd> as FullContext>::from_lua(lua).unwrap()
    }
}

unsafe impl<'a, 'd: 'a> FullContext<'a> for WorldFragment<'a, 'd> {
    unsafe fn check(lua: &'a mut LuaState) {
        // Run get_ctx to check that the context is available.
        let ptr = get_ctx(lua);
        check_ctx_flags::<Self>(lua, ptr);
    }

    unsafe fn from_lua(lua: &'a mut LuaState) -> WorldFragment<'a, 'd> {
        let ptr = get_ctx_raw(lua);
        assert!(!ptr.is_null());
        cast_ctx(ptr)
    }
}

engine_part_typedef!(OnlyWorld(world));
unsafe impl<'a, 'd: 'a> PartialContext for &'a world::World<'d> {
    unsafe fn from_lua(lua: &mut LuaState) -> &'a world::World<'d> {
        // Transmute away the lifetime errors
        mem::transmute(get_cast_ctx::<OnlyWorld>(lua).world())
    }
}

engine_part_typedef!(OnlyMessages(messages));
unsafe impl<'a> PartialContext for &'a mut messages::Messages {
    unsafe fn from_lua(lua: &mut LuaState) -> &'a mut messages::Messages {
        mem::transmute(get_cast_ctx::<OnlyMessages>(lua).messages_mut())
    }
}

engine_part_typedef!(OnlyTimer(timer));
unsafe impl<'a> PartialContext for &'a mut timer::Timer {
    unsafe fn from_lua(lua: &mut LuaState) -> &'a mut timer::Timer {
        mem::transmute(get_cast_ctx::<OnlyTimer>(lua).timer_mut())
    }
}

engine_part_typedef!(OnlyTerrainGen(terrain_gen));
unsafe impl<'a, 'd: 'a> PartialContext for &'a terrain_gen::TerrainGen<'d> {
    unsafe fn from_lua(lua: &mut LuaState) -> &'a terrain_gen::TerrainGen<'d> {
        mem::transmute(get_cast_ctx::<OnlyTerrainGen>(lua).terrain_gen())
    }
}

engine_part_typedef!(EmptyPart());
unsafe impl<'a, 'd: 'a> PartialContext for Time {
    unsafe fn from_lua(lua: &mut LuaState) -> Time {
        get_cast_ctx::<EmptyPart>(lua).now()
    }
}
