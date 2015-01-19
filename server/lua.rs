use std::ffi::CString;
use std::marker::{ContravariantLifetime, NoCopy};
use std::mem;
use std::ptr;

use libc;
use libc::{c_void, c_int, c_char, size_t};

use self::ffi::{lua_State, lua_Alloc, lua_CFunction, lua_Integer, lua_Reader};

mod ffi {
    #![allow(non_camel_case_types)]
    use libc::{c_void, c_int, c_char, size_t, ptrdiff_t};

    pub type lua_State = c_void;

    pub type lua_Alloc = fn(*mut c_void, *mut c_void, size_t, size_t) -> *mut c_void;
    pub type lua_CFunction = fn(*mut lua_State) -> c_int;
    pub type lua_Integer = ptrdiff_t;
    pub type lua_Reader = fn(*mut lua_State, data: *mut c_void, size: *mut size_t) -> *const c_char;

    #[link(name = "lua5.1")]
    extern "C" {
        pub fn lua_newstate(f: lua_Alloc, ud: *mut c_void) -> *mut lua_State;
        pub fn luaL_newstate() -> *mut lua_State;
        pub fn lua_close(L: *mut lua_State);

        pub fn lua_load(L: *mut lua_State, reader: lua_Reader, data: *mut c_void, chunkname: *const c_char);
        pub fn lua_pcall(L: *mut lua_State, nargs: c_int, nresults: c_int, errfunc: c_int) -> c_int;
        pub fn lua_error(L: *mut lua_State) -> !;

        pub fn luaL_openlibs(L: *mut lua_State);
        pub fn luaL_loadfile(L: *mut lua_State, filename: *const c_char) -> c_int;

        pub fn lua_createtable(L: *mut lua_State, narr: c_int, nrec: c_int);
        pub fn lua_newuserdata(L: *mut lua_State, size: size_t) -> *mut c_void;
        pub fn lua_pushcclosure(L: *mut lua_State, f: lua_CFunction, n: c_int);
        pub fn lua_pushinteger(L: *mut lua_State, i: lua_Integer);
        pub fn lua_pushnil(L: *mut lua_State);
        pub fn lua_pushlightuserdata(L: *mut lua_State, p: *mut c_void);
        pub fn lua_pushlstring(L: *mut lua_State, s: *const c_char, len: size_t);

        pub fn lua_tointeger(L: *mut lua_State, index: c_int) -> lua_Integer;
        pub fn lua_tolstring(L: *mut lua_State, index: c_int, len: *mut size_t) -> *const c_char;
        pub fn lua_topointer(L: *mut lua_State, index: c_int) -> *mut c_void;
        pub fn lua_touserdata(L: *mut lua_State, index: c_int) -> *mut c_void;

        pub fn lua_gettop(L: *mut lua_State) -> c_int;
        pub fn lua_settop(L: *mut lua_State, index: c_int);
        pub fn lua_insert(L: *mut lua_State, index: c_int);
        pub fn lua_replace(L: *mut lua_State, index: c_int);
        pub fn lua_pushvalue(L: *mut lua_State, index: c_int);

        pub fn lua_gettable(L: *mut lua_State, t: c_int);
        pub fn lua_settable(L: *mut lua_State, t: c_int);
        pub fn lua_rawget(L: *mut lua_State, index: c_int);
        pub fn lua_rawset(L: *mut lua_State, index: c_int);
        pub fn lua_rawgeti(L: *mut lua_State, index: c_int, n: c_int);
        pub fn lua_rawseti(L: *mut lua_State, index: c_int, n: c_int);

        pub fn lua_getmetatable(L: *mut lua_State, index: c_int);
        pub fn lua_setmetatable(L: *mut lua_State, index: c_int);

        pub fn luaL_ref(L: *mut lua_State, t: c_int) -> c_int;
        pub fn luaL_unref(L: *mut lua_State, t: c_int, r: c_int);

        pub fn lua_rawequal(L: *mut lua_State, index1: c_int, index2: c_int) -> c_int;

        pub fn lua_type(L: *mut lua_State, index: c_int) -> c_int;
    }
}


#[unsafe_no_drop_flag]
pub struct OwnedLuaState {
    L: *mut lua_State,
}

impl OwnedLuaState {
    pub fn new() -> LuaResult<OwnedLuaState> {
        let L = unsafe { ffi::luaL_newstate() };

        if L.is_null() {
            Err((ErrorType::ErrMem, String::from_str("failed to allocate memory")))
        } else {
            Ok(OwnedLuaState {
                L: L,
            })
        }
    }

    pub fn get<'a>(&'a mut self) -> LuaState<'a> {
        unsafe { LuaState::new(self.L) }
    }
}

impl Drop for OwnedLuaState {
    fn drop(&mut self) {
        if self.L.is_null() {
            return;
        }

        unsafe { ffi::lua_close(self.L) };
        self.L = ptr::null_mut();
    }
}

fn lua_alloc(_userdata: *mut c_void,
             ptr: *mut c_void,
             old_size: size_t,
             new_size: size_t) -> *mut c_void {
    if new_size == 0 {
        unsafe { libc::free(ptr) };
        ptr::null_mut()
    } else {
        // NB: ptr is guaranteed to be null when requesting a new allocation (i.e., when old_size
        // is 0).
        unsafe { libc::realloc(ptr, new_size) }
    }
}


#[derive(Copy, PartialEq, Eq, Show)]
pub enum ErrorType {
    Yield = 1,
    ErrRun = 2,
    ErrSyntax = 3,
    ErrMem = 4,
    ErrErr = 5,
    ErrFile = 6,
    ErrUnknown,
}

impl ErrorType {
    pub fn from_code(code: c_int) -> ErrorType {
        match code {
            1 => ErrorType::Yield,
            2 => ErrorType::ErrRun,
            3 => ErrorType::ErrSyntax,
            4 => ErrorType::ErrMem,
            5 => ErrorType::ErrErr,
            6 => ErrorType::ErrFile,
            _ => ErrorType::ErrUnknown,
        }
    }
}

type Error = (ErrorType, String);

type LuaResult<T> = Result<T, Error>;

fn make_result(lua: &mut LuaState, code: c_int) -> LuaResult<()> {
    if code == 0 {
        Ok(())
    } else {
        let ty = ErrorType::from_code(code);
        let msg = lua.to_string(-1).unwrap_or(String::from_str("(no message)"));
        Err((ty, msg))
    }
}


#[derive(Copy, PartialEq, Eq, Show)]
pub enum ValueType {
    Nil = 0,
    Boolean = 1,
    LightUserdata = 2,
    Number = 3,
    String = 4,
    Table = 5,
    Function = 6,
    Userdata = 7,
    Thread = 8,
    Unknown,
}

impl ValueType {
    pub fn from_code(code: c_int) -> ValueType {
        match code {
            0 => ValueType::Nil,
            1 => ValueType::Boolean,
            2 => ValueType::LightUserdata,
            3 => ValueType::Number,
            4 => ValueType::String,
            5 => ValueType::Table,
            6 => ValueType::Function,
            7 => ValueType::Userdata,
            8 => ValueType::Thread,
            _ => ValueType::Unknown,
        }
    }
}


pub const MULTRET: c_int = -1;
pub const PSEUDO_INDEX_LIMIT: c_int = -10000;
pub const REGISTRY_INDEX: c_int = -10000;
pub const ENVIRON_INDEX: c_int = -10001;
pub const GLOBALS_INDEX: c_int = -10002;


pub type lua_RustFunction = fn(LuaState) -> c_int;

pub struct LuaState<'a> {
    L: *mut lua_State,
    _marker1: ContravariantLifetime<'a>,
    _marker2: NoCopy,
}

unsafe fn _static_assertions() {
    let x = mem::transmute::<*mut lua_State, LuaState>(mem::zeroed());
}

impl<'a> LuaState<'a> {
    pub unsafe fn new(L: *mut lua_State) -> LuaState<'a> {
        LuaState {
            L: L,
            _marker1: ContravariantLifetime,
            _marker2: NoCopy,
        }
    }

    // Basic stack manipulation

    pub fn insert(&mut self, index: c_int) {
        unsafe { ffi::lua_insert(self.L, index) };
    }

    pub fn replace(&mut self, index: c_int) {
        unsafe { ffi::lua_replace(self.L, index) };
    }

    pub fn copy(&mut self, index: c_int) {
        unsafe { ffi::lua_pushvalue(self.L, index) };
    }

    pub fn pop(&mut self, count: c_int) {
        unsafe { ffi::lua_settop(self.L, -count - 1) };
    }

    pub fn top_index(&mut self) -> c_int {
        unsafe { ffi::lua_gettop(self.L) }
    }

    // Pushing values onto the stack

    pub fn push_integer(&mut self, i: isize) {
        unsafe { ffi::lua_pushinteger(self.L, i as lua_Integer) };
    }

    pub fn push_nil(&mut self) {
        unsafe { ffi::lua_pushnil(self.L) };
    }

    pub fn push_light_userdata<T>(&mut self, ptr: *mut T) {
        unsafe { ffi::lua_pushlightuserdata(self.L, ptr as *mut c_void) };
    }

    pub fn push_rust_function(&mut self, f: lua_RustFunction) {
        let f = unsafe { mem::transmute(f) };
        unsafe { ffi::lua_pushcclosure(self.L, f, 0) };
    }

    pub fn push_string(&mut self, s: &str) {
        let ptr = s.as_ptr() as *const c_char;
        let len = s.len() as size_t;
        // Make sure the conversion to c_int didn't overflow or underflow.
        assert!(len >= 0 && len as usize == s.len());
        unsafe { ffi::lua_pushlstring(self.L, ptr, len) };
    }

    pub fn push_table(&mut self) {
        self.push_table_prealloc(0, 0);
    }

    pub fn push_table_prealloc(&mut self, arr_count: c_int, named_count: c_int) {
        unsafe { ffi::lua_createtable(self.L, arr_count, named_count) };
    }

    pub fn push_userdata<T: Copy>(&mut self, value: T) {
        let raw_size = mem::size_of::<T>();
        let size = raw_size as size_t;
        assert!(size >= 0 && size as usize == raw_size);
        let ptr = unsafe { ffi::lua_newuserdata(self.L, size) } as *mut T;
        unsafe { *ptr = value; }
    }

    // Reading values from the stack

    // NB: If the target item is a number, lua_tolstring will replace it on the stack with its
    // string representation.  Hence we require `&mut self` instead of `&self` here.
    pub fn to_bytes(&mut self, index: c_int) -> Option<Vec<u8>> {
        let mut len = 0;
        let ptr = unsafe { ffi::lua_tolstring(self.L, index, &mut len as *mut _) };
        if ptr.is_null() {
            None
        } else {
            let vec = unsafe { Vec::from_raw_buf(ptr as *const u8, len as usize) };
            Some(vec)
        }
    }

    pub fn to_integer(&self, index: c_int) -> isize {
        unsafe { ffi::lua_tointeger(self.L, index) as isize }
    }

    pub fn to_string(&mut self, index: c_int) -> Option<String> {
        let v = match self.to_bytes(index) {
            Some(v) => v,
            None => return None,
        };
        match String::from_utf8(v) {
            Ok(s) => Some(s),
            Err(_) => None,
        }
    }

    pub unsafe fn to_userdata<'b, T>(&'b self, index: c_int) -> Option<&'b T> {
        unsafe {
            let ptr = ffi::lua_touserdata(self.L, index);
            if ptr.is_null() {
                None
            } else {
                Some(mem::transmute(ptr))
            }
        }
    }

    pub unsafe fn to_userdata_mut<'b, T>(&'b mut self, index: c_int) -> Option<&'b mut T> {
        unsafe {
            let ptr = ffi::lua_touserdata(self.L, index);
            if ptr.is_null() {
                None
            } else {
                Some(mem::transmute(ptr))
            }
        }
    }

    pub unsafe fn to_userdata_raw<T>(&self, index: c_int) -> *mut T {
        unsafe { ffi::lua_touserdata(self.L, index) as *mut T }
    }

    // Table manipulation

    pub fn get_table(&mut self, index: c_int) {
        unsafe { ffi::lua_gettable(self.L, index) };
    }

    pub fn set_table(&mut self, index: c_int) {
        unsafe { ffi::lua_settable(self.L, index) };
    }

    pub fn get_table_raw(&mut self, index: c_int) {
        unsafe { ffi::lua_rawget(self.L, index) };
    }

    pub fn set_table_raw(&mut self, index: c_int) {
        unsafe { ffi::lua_rawset(self.L, index) };
    }

    pub fn get_field(&mut self, index: c_int, k: &str) {
        self.push_string(k);
        if index < 0 && index > PSEUDO_INDEX_LIMIT {
            self.get_table(index - 1);
        } else {
            self.get_table(index);
        }
    }

    pub fn set_field(&mut self, index: c_int, k: &str) {
        self.push_string(k);
        self.insert(-2);
        if index < 0 && index > PSEUDO_INDEX_LIMIT {
            self.set_table(index - 1);
        } else {
            self.set_table(index);
        }
    }

    pub fn get_metatable(&mut self, index: c_int) {
        unsafe { ffi::lua_getmetatable(self.L, index) };
    }

    pub fn set_metatable(&mut self, index: c_int) {
        unsafe { ffi::lua_setmetatable(self.L, index) };
    }

    // Registry slots

    pub fn alloc_slot(&mut self) -> c_int {
        // Need a non-nil value, else we will get a bogus slot index.
        self.push_integer(0);
        let slot = unsafe { ffi::luaL_ref(self.L, REGISTRY_INDEX) };

        self.push_nil();
        self.set_slot(slot);
        slot
    }

    pub fn free_slot(&mut self, slot: c_int) {
        unsafe { ffi::luaL_unref(self.L, REGISTRY_INDEX, slot) };
    }

    pub fn get_slot(&mut self, slot: c_int) {
        unsafe { ffi::lua_rawgeti(self.L, REGISTRY_INDEX, slot) };
    }

    pub fn set_slot(&mut self, slot: c_int) {
        unsafe { ffi::lua_rawseti(self.L, REGISTRY_INDEX, slot) };
    }

    // Calling functions

    pub fn pcall(&mut self, num_args: c_int, num_results: c_int, err_func: c_int) -> LuaResult<()> {
        let code = unsafe { ffi::lua_pcall(self.L, num_args, num_results, err_func) };
        make_result(self, code)
    }

    // Miscellaneous

    pub fn open_libs(&mut self) {
        unsafe { ffi::luaL_openlibs(self.L) };
    }

    pub fn register(&mut self, name: &str, f: lua_RustFunction) {
        self.push_string(name);
        self.push_rust_function(f);
        self.set_table(GLOBALS_INDEX);
    }

    pub fn load_file(&mut self, path: &Path) -> LuaResult<()> {
        // NB: from_slice will panic if the path contains null characters, but that's impossible
        // because Path checks for \0 and rejects the string if found.
        let s = CString::from_slice(path.as_vec());
        let code = unsafe { ffi::luaL_loadfile(self.L, s.as_slice_with_nul().as_ptr()) };
        make_result(self, code)
    }

    pub unsafe fn error(&mut self) -> ! {
        ffi::lua_error(self.L);
    }

    pub fn raw_equal(&mut self, index1: c_int, index2: c_int) -> bool {
        let result = unsafe { ffi::lua_rawequal(self.L, index1, index2) };
        result != 0
    }

    pub fn dump_stack(&mut self, msg: &str) {
        info!("lua stack dump - {}", msg);
        for i in range(1, self.top_index() + 1) {
            let ty_code = unsafe { ffi::lua_type(self.L, i) };
            let ty = ValueType::from_code(ty_code);

            let desc = match ty {
                ValueType::Nil =>
                    format!("nil"),
                ValueType::Boolean |
                ValueType::Number |
                ValueType::String =>
                    format!("{:?}", self.to_string(i)),
                ValueType::LightUserdata |
                ValueType::Table |
                ValueType::Function |
                ValueType::Userdata |
                ValueType::Thread =>
                    format!("{:p}", unsafe { ffi::lua_topointer(self.L, i) }),
                ValueType::Unknown =>
                    format!("(unknown)"),
            };
            info!("{}: {:?} {}", i, ty, desc);
        }
    }
}
