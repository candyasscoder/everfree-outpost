use std::error;
use std::fmt;
use std::io;
use std::mem;
use std::result;
use libc::c_int;

use types::*;

use engine;
use engine::glue::HiddenWorldFragment;
use lua::{self, LuaState, ValueType, REGISTRY_INDEX};
use util::Convert;
use util::StrError;
use world::{self, World, Client, Entity, Inventory, Plane, TerrainChunk, Structure};
use world::StructureFlags;
use world::object::*;
use world::save::{self, Writer, Reader};

use super::traits::{ToLua, Userdata, metatable_key};
use super::userdata;


#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Str(StrError),
    Lua(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Io(ref e) => e.fmt(f),
            Error::Str(ref e) => e.fmt(f),
            Error::Lua(ref e) => e.fmt(f),
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Io(ref e) => e.description(),
            Error::Str(ref e) => e.description(),
            Error::Lua(ref s) => &**s,
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::Io(ref e) => Some(e as &error::Error),
            Error::Str(ref e) => Some(e as &error::Error),
            Error::Lua(_) => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Error {
        Error::Io(e)
    }
}

impl From<StrError> for Error {
    fn from(e: StrError) -> Error {
        Error::Str(e)
    }
}

impl From<save::Error> for Error {
    fn from(e: save::Error) -> Error {
        match e {
            save::Error::Io(e) => Error::Io(e),
            save::Error::Str(e) => Error::Str(e),
        }
    }
}

impl<'a> From<(lua::ErrorType, &'a str)> for Error {
    fn from((et, s): (lua::ErrorType, &'a str)) -> Error {
        Error::Lua(format!("{:?}: {}", et, s))
    }
}

impl<'a> From<Error> for save::Error {
    fn from(e: Error) -> save::Error {
        match e {
            Error::Io(e) => save::Error::Io(e),
            Error::Str(e) => save::Error::Str(e),
            Error::Lua(msg) => {
                warn!("lua error: {}", msg);
                save::Error::Str(StrError {
                    msg: "error while running lua callback",
                })
            },
        }
    }
}

pub type Result<T> = result::Result<T, Error>;



macro_rules! primitive_enum {
    (enum $name:ident: $prim:ty { $($variant:ident = $disr:expr,)* }) => {
        #[derive(Clone, Copy, PartialEq, Eq, Debug)]
        enum $name {
            $($variant = $disr,)*
        }

        impl $name {
            pub fn from_primitive(x: $prim) -> Option<$name> {
                match x {
                    $( $disr => Some($name::$variant), )*
                    _ => None,
                }
            }
        }
    };
}

primitive_enum! {
    enum Tag: u8 {
        Nil =           0x00,
        Bool =          0x01,
        SmallInt =      0x02,
        LargeInt =      0x03,
        Float =         0x04,
        SmallString =   0x05,
        LargeString =   0x06,
        Table =         0x07,

        World =         0x10,
        Client =        0x11,
        Entity =        0x12,
        Inventory =     0x13,
        Structure =     0x14,

        StableClient =      0x20,
        StableEntity =      0x21,
        StableInventory =   0x22,
        StablePlane =       0x23,
        StableStructure =   0x24,

        V3 =            0x30,
        TimeU =         0x31,
    }
}


pub type WriteHooks<'a, 'd> = ::engine::glue::SaveWriteHooks<'a, 'd>;

impl<'a, 'd> save::WriteHooks for WriteHooks<'a, 'd> {
    fn post_write_world<W: Writer>(&mut self,
                                   writer: &mut W,
                                   _w: &World) -> save::Result<()> {
        try!(write_extra(self.script_mut().owned_lua.get(), writer, |lua| {
            lua.get_field(REGISTRY_INDEX, "outpost_callback_get_world_extra");
            try!(lua.pcall(0, 1, 0));
            Ok(())
        }));
        Ok(())
    }

    fn post_write_client<W: Writer>(&mut self,
                                    writer: &mut W,
                                    c: &ObjectRef<Client>) -> save::Result<()> {
        try!(write_extra(self.script_mut().owned_lua.get(), writer, |lua| {
            call_get_extra(lua, "outpost_callback_get_client_extra", c.id().unwrap())
        }));
        Ok(())
    }

    fn post_write_entity<W: Writer>(&mut self,
                                    writer: &mut W,
                                    e: &ObjectRef<Entity>) -> save::Result<()> {
        try!(write_extra(self.script_mut().owned_lua.get(), writer, |lua| {
            call_get_extra(lua, "outpost_callback_get_entity_extra", e.id().unwrap())
        }));
        Ok(())
    }

    fn post_write_inventory<W: Writer>(&mut self,
                                       writer: &mut W,
                                       i: &ObjectRef<Inventory>) -> save::Result<()> {
        try!(write_extra(self.script_mut().owned_lua.get(), writer, |lua| {
            call_get_extra(lua, "outpost_callback_get_inventory_extra", i.id().unwrap())
        }));
        Ok(())
    }

    fn post_write_plane<W: Writer>(&mut self,
                                   writer: &mut W,
                                   p: &ObjectRef<Plane>) -> save::Result<()> {
        try!(write_extra(self.script_mut().owned_lua.get(), writer, |lua| {
            call_get_extra(lua, "outpost_callback_get_plane_extra", p.id().unwrap())
        }));
        Ok(())
    }

    fn post_write_terrain_chunk<W: Writer>(&mut self,
                                           _writer: &mut W,
                                           _t: &ObjectRef<TerrainChunk>) -> save::Result<()> {
        // TODO: support terrain_chunk script data
        Ok(())
    }

    fn post_write_structure<W: Writer>(&mut self,
                                       writer: &mut W,
                                       s: &ObjectRef<Structure>) -> save::Result<()> {
        if s.flags().contains(world::flags::S_HAS_SAVE_HOOKS) {
            try!(self.call_unload_hook("outpost_callback_structure_unload",
                                       s.id().unwrap()));
        }
        try!(write_extra(self.script_mut().owned_lua.get(), writer, |lua| {
            call_get_extra(lua, "outpost_callback_get_structure_extra", s.id().unwrap())
        }));
        Ok(())
    }
}

impl<'a, 'd> WriteHooks<'a, 'd> {
    fn call_unload_hook<T: ToLua>(&mut self, func: &str, id: T) -> Result<()> {
        // FIXME: SUPER UNSAFE!!!  This allows scripts to access engine parts they shouldn't have
        // access to in this context.
        let ptr: *mut engine::Engine = unsafe { mem::transmute_copy(self) };
        self.script_mut().with_context(ptr, |lua| -> Result<()> {
            lua.get_field(REGISTRY_INDEX, func);
            id.to_lua(lua);
            try!(lua.pcall(1, 0, 0));
            Ok(())
        })
    }
}

fn call_get_extra<T: ToLua>(lua: &mut LuaState, func: &str, id: T) -> Result<()> {
    lua.get_field(REGISTRY_INDEX, func);
    id.to_lua(lua);
    try!(lua.pcall(1, 1, 0));
    Ok(())
}

fn write_extra<W: Writer, F>(mut lua: LuaState, w: &mut W, get_extra: F) -> Result<()> 
        where F: FnOnce(&mut LuaState) -> Result<()> {
    try!(get_extra(&mut lua));
    try!(write_value(&mut lua, w, -1));
    lua.pop(1);
    Ok(())
}

fn write_value<W: Writer>(lua: &mut LuaState, w: &mut W, index: c_int) -> Result<()> {
    match lua.type_of(index) {
        ValueType::Nil => {
            try!(w.write(Tag::Nil as u8));
        },

        ValueType::Boolean => {
            let b = lua.to_boolean(index);
            try!(w.write((Tag::Bool as u8, b as u8)));
        },

        ValueType::Number => {
            let n = lua.to_number(index);
            let i = n as i32;
            if i as f64 == n {
                match i.to_i16() {
                    Some(small_i) => {
                        try!(w.write((Tag::SmallInt as u8, 0u8, small_i)));
                    },
                    None => {
                        try!(w.write(Tag::LargeInt as u8));
                        try!(w.write(i));
                    },
                }
            } else {
                try!(w.write(Tag::Float as u8));
                try!(w.write(n));
            }
        },

        ValueType::String => {
            let s = unwrap!(lua.to_string(index));
            match s.len().to_u16() {
                Some(small_len) => {
                    try!(w.write((Tag::SmallString as u8, 0u8, small_len)));
                    try!(w.write_str_bytes(s));
                },
                None => {
                    try!(w.write(Tag::LargeString as u8));
                    try!(w.write_str(s));
                },
            }
        },

        ValueType::Table => {
            try!(w.write(Tag::Table as u8));
            try!(write_table(lua, w, index));
        },

        ValueType::Userdata => {
            // The tag is handled by write_userdata, since it varies depending on the userdata
            // type.
            try!(write_userdata(lua, w, index));
        },

        ty => warn!("don't know how to serialize {:?}", ty),
    }

    Ok(())
}

fn write_table<W: Writer>(lua: &mut LuaState, w: &mut W, index: c_int) -> Result<()> {
    let index = lua.abs_index(index);

    lua.push_nil();
    while lua.next_entry(index) {
        try!(write_value(lua, w, -2));
        try!(write_value(lua, w, -1));
        lua.pop(1);
    }

    lua.push_nil();
    try!(write_value(lua, w, -1));
    lua.pop(1);

    Ok(())
}

fn write_userdata<W: Writer>(lua: &mut LuaState, w: &mut W, index: c_int) -> Result<()> {
    let index = lua.abs_index(index);

    lua.get_metatable(index);
    fn get_userdata_opt<'a, U: Userdata>(lua: &'a mut LuaState, index: c_int) -> Option<&'a U> {
        lua.get_field(REGISTRY_INDEX, metatable_key::<U>());
        if lua.raw_equal(-1, -2) {
            lua.pop(2);
            unsafe { lua.to_userdata(index) }
        } else {
            lua.pop(1);
            None
        }
    }

    // Can't use `else if let` here because it would produce overlapping borrows.
    if let Some(_) = get_userdata_opt::<userdata::world::World>(lua, index) {
        try!(w.write(Tag::World as u8));
        return Ok(());
    }

    if let Some(c) = get_userdata_opt::<userdata::world::Client>(lua, index) {
        try!(w.write(Tag::Client as u8));
        try!(w.write_id(c.id));
        return Ok(());
    }
    if let Some(e) = get_userdata_opt::<userdata::world::Entity>(lua, index) {
        try!(w.write(Tag::Entity as u8));
        try!(w.write_id(e.id));
        return Ok(());
    }
    if let Some(i) = get_userdata_opt::<userdata::world::Inventory>(lua, index) {
        try!(w.write(Tag::Inventory as u8));
        try!(w.write_id(i.id));
        return Ok(());
    }
    if let Some(s) = get_userdata_opt::<userdata::world::Structure>(lua, index) {
        try!(w.write(Tag::Structure as u8));
        try!(w.write_id(s.id));
        return Ok(());
    }

    if let Some(c) = get_userdata_opt::<userdata::world::StableClient>(lua, index) {
        try!(w.write(Tag::StableClient as u8));
        try!(w.write(c.id.val));
        return Ok(());
    }
    if let Some(e) = get_userdata_opt::<userdata::world::StableEntity>(lua, index) {
        try!(w.write(Tag::StableEntity as u8));
        try!(w.write(e.id.val));
        return Ok(());
    }
    if let Some(i) = get_userdata_opt::<userdata::world::StableInventory>(lua, index) {
        try!(w.write(Tag::StableInventory as u8));
        try!(w.write(i.id.val));
        return Ok(());
    }
    if let Some(s) = get_userdata_opt::<userdata::world::StablePlane>(lua, index) {
        try!(w.write(Tag::StablePlane as u8));
        try!(w.write(s.id.val));
        return Ok(());
    }
    if let Some(s) = get_userdata_opt::<userdata::world::StableStructure>(lua, index) {
        try!(w.write(Tag::StableStructure as u8));
        try!(w.write(s.id.val));
        return Ok(());
    }

    if let Some(v) = get_userdata_opt::<V3>(lua, index) {
        try!(w.write(Tag::V3 as u8));
        try!(w.write((v.x, v.y, v.z)));
        return Ok(());
    }
    if let Some(t) = get_userdata_opt::<userdata::timer::TimeU>(lua, index) {
        try!(w.write(Tag::TimeU as u8));
        try!(w.write(t.t));
        return Ok(());
    }

    warn!("don't know how to serialize unrecognized userdata");
    lua.pop(1);
    Ok(())
}


pub type ReadHooks<'a, 'd> = ::engine::glue::SaveReadHooks<'a, 'd>;

impl<'a, 'd> save::ReadHooks for ReadHooks<'a, 'd> {
    fn post_read_world<R: Reader>(&mut self,
                                  reader: &mut R) -> save::Result<()> {
        try!(self.read_extra(reader, |lua| {
            lua.get_field(REGISTRY_INDEX, "outpost_callback_set_world_extra");
        }));
        Ok(())
    }

    fn post_read_client<R: Reader>(&mut self,
                                   reader: &mut R,
                                   cid: ClientId) -> save::Result<()> {
        try!(self.read_extra(reader, |lua| {
            push_setter_and_id(lua, "outpost_callback_set_client_extra", cid.unwrap())
        }));
        Ok(())
    }

    fn post_read_entity<R: Reader>(&mut self,
                                   reader: &mut R,
                                   eid: EntityId) -> save::Result<()> {
        try!(self.read_extra(reader, |lua| {
            push_setter_and_id(lua, "outpost_callback_set_entity_extra", eid.unwrap())
        }));
        Ok(())
    }

    fn post_read_inventory<R: Reader>(&mut self,
                                      reader: &mut R,
                                      iid: InventoryId) -> save::Result<()> {
        try!(self.read_extra(reader, |lua| {
            push_setter_and_id(lua, "outpost_callback_set_inventory_extra", iid.unwrap())
        }));
        Ok(())
    }

    fn post_read_plane<R: Reader>(&mut self,
                                  reader: &mut R,
                                  pid: PlaneId) -> save::Result<()> {
        try!(self.read_extra(reader, |lua| {
            push_setter_and_id(lua, "outpost_callback_set_plane_extra", pid.unwrap())
        }));
        Ok(())
    }

    fn post_read_terrain_chunk<R: Reader>(&mut self,
                                          _reader: &mut R,
                                          _tcid: TerrainChunkId) -> save::Result<()> {
        // TODO: support terrain_chunk script data
        Ok(())
    }

    fn post_read_structure<R: Reader>(&mut self,
                                      reader: &mut R,
                                      sid: StructureId,
                                      flags: StructureFlags) -> save::Result<()> {
        try!(self.read_extra(reader, |lua| {
            push_setter_and_id(lua, "outpost_callback_set_structure_extra", sid.unwrap())
        }));
        if flags.contains(world::flags::S_HAS_SAVE_HOOKS) {
            try!(self.call_load_hook("outpost_callback_structure_load",
                                     sid.unwrap()));
        }
        Ok(())
    }

    // Cleanup functions are simpler because they interact only with the ScriptEngine.  The
    // post_read functions have to be able to read SaveIds, which may require mutating the World.
    fn cleanup_world(&mut self) -> save::Result<()> {
        try!(clear_extra(self.script_mut().owned_lua.get(), |lua| {
            lua.get_field(REGISTRY_INDEX, "outpost_callback_set_world_extra");
        }));
        Ok(())
    }

    fn cleanup_client(&mut self, cid: ClientId) -> save::Result<()> {
        try!(clear_extra(self.script_mut().owned_lua.get(), |lua| {
            push_setter_and_id(lua, "outpost_callback_set_client_extra", cid.unwrap())
        }));
        Ok(())
    }

    fn cleanup_entity(&mut self, eid: EntityId) -> save::Result<()> {
        try!(clear_extra(self.script_mut().owned_lua.get(), |lua| {
            push_setter_and_id(lua, "outpost_callback_set_entity_extra", eid.unwrap())
        }));
        Ok(())
    }

    fn cleanup_inventory(&mut self, iid: InventoryId) -> save::Result<()> {
        try!(clear_extra(self.script_mut().owned_lua.get(), |lua| {
            push_setter_and_id(lua, "outpost_callback_set_inventory_extra", iid.unwrap())
        }));
        Ok(())
    }

    fn cleanup_plane(&mut self, pid: PlaneId) -> save::Result<()> {
        try!(clear_extra(self.script_mut().owned_lua.get(), |lua| {
            push_setter_and_id(lua, "outpost_callback_set_plane_extra", pid.unwrap())
        }));
        Ok(())
    }

    fn cleanup_terrain_chunk(&mut self, _tcid: TerrainChunkId) -> save::Result<()> {
        // TODO: support terrain_chunk script data
        Ok(())
    }

    fn cleanup_structure(&mut self, sid: StructureId) -> save::Result<()> {
        try!(clear_extra(self.script_mut().owned_lua.get(), |lua| {
            push_setter_and_id(lua, "outpost_callback_set_structure_extra", sid.unwrap())
        }));
        Ok(())
    }
}

impl<'a, 'd> ReadHooks<'a, 'd> {
    fn lua(&mut self) -> LuaState {
        self.script_mut().owned_lua.get()
    }

    fn with_lua<F, R>(&mut self, f: F) -> R
            where F: FnOnce(&mut LuaState) -> R {
        let mut lua = self.script_mut().owned_lua.get();
        f(&mut lua)
    }

    fn wf<'b>(&'b mut self) -> HiddenWorldFragment<'b, 'd> {
        self.as_hidden_world_fragment()
    }

    fn call_load_hook<T: ToLua>(&mut self, func: &str, id: T) -> Result<()> {
        // FIXME: SUPER UNSAFE!!!  This allows scripts to access engine parts they shouldn't have
        // access to in this context.
        let ptr: *mut engine::Engine = unsafe { mem::transmute_copy(self) };
        self.script_mut().with_context(ptr, |lua| -> Result<()> {
            lua.get_field(REGISTRY_INDEX, func);
            id.to_lua(lua);
            try!(lua.pcall(1, 0, 0));
            Ok(())
        })
    }

    fn read_extra<R: Reader, F>(&mut self,
                                r: &mut R,
                                push_setter: F) -> Result<()>
            where F: FnOnce(&mut LuaState) {
        let base = self.with_lua(|lua| {
            let base = lua.top_index();
            push_setter(lua);
            base
        });
        try!(self.read_value(r));
        self.with_lua(|lua| {
            let args = lua.top_index() - base - 1;
            lua.pcall(args, 0, 0)
               .map_err(From::from)
        })
    }

    fn read_value<R: Reader>(&mut self,
                             r: &mut R) -> Result<()> {
        let (tag, a, b): (u8, u8, u16) = try!(r.read());
        let tag = unwrap!(Tag::from_primitive(tag));
        match tag {
            Tag::Nil => {
                self.lua().push_nil();
            },
            Tag::Bool => {
                self.lua().push_boolean(a != 0);
            },
            Tag::SmallInt => {
                self.lua().push_integer(b as i16 as isize);
            },
            Tag::LargeInt => {
                let i: i32 = try!(r.read());
                self.lua().push_integer(i as isize);
            },
            Tag::Float => {
                let f: f64 = try!(r.read());
                self.lua().push_number(f);
            },
            Tag::SmallString => {
                let s = try!(r.read_str_bytes(b as usize));
                self.lua().push_string(&*s);
            },
            Tag::LargeString => {
                let s = try!(r.read_str());
                self.lua().push_string(&*s);
            },
            Tag::Table => {
                try!(self.read_table(r));
            },

            Tag::World => {
                let w = userdata::world::World;
                w.to_lua(&mut self.lua());
            },

            Tag::Client => {
                let cid = try!(r.read_id(&mut self.wf()));
                let c = userdata::world::Client { id: cid };
                c.to_lua(&mut self.lua());
            },
            Tag::Entity => {
                let eid = try!(r.read_id(&mut self.wf()));
                let e = userdata::world::Entity { id: eid };
                e.to_lua(&mut self.lua());
            },
            Tag::Inventory => {
                let iid = try!(r.read_id(&mut self.wf()));
                let i = userdata::world::Inventory { id: iid };
                i.to_lua(&mut self.lua());
            },
            Tag::Structure => {
                let sid = try!(r.read_id(&mut self.wf()));
                let s = userdata::world::Structure { id: sid };
                s.to_lua(&mut self.lua());
            },

            Tag::StableClient => {
                let cid = try!(r.read());
                let c = userdata::world::StableClient { id: Stable::new(cid) };
                c.to_lua(&mut self.lua());
            },
            Tag::StableEntity => {
                let eid = try!(r.read());
                let e = userdata::world::StableEntity { id: Stable::new(eid) };
                e.to_lua(&mut self.lua());
            },
            Tag::StableInventory => {
                let iid = try!(r.read());
                let i = userdata::world::StableInventory { id: Stable::new(iid) };
                i.to_lua(&mut self.lua());
            },
            Tag::StablePlane => {
                let sid = try!(r.read());
                let s = userdata::world::StablePlane { id: Stable::new(sid) };
                s.to_lua(&mut self.lua());
            },
            Tag::StableStructure => {
                let sid = try!(r.read());
                let s = userdata::world::StableStructure { id: Stable::new(sid) };
                s.to_lua(&mut self.lua());
            },

            Tag::V3 => {
                let (x, y, z) = try!(r.read());
                V3::new(x, y, z).to_lua(&mut self.lua());
            },
            Tag::TimeU => {
                let t = try!(r.read());
                let tu = userdata::timer::TimeU { t: t };
                tu.to_lua(&mut self.lua());
            },
        }
        Ok(())
    }

    fn read_table<R: Reader>(&mut self,
                             r: &mut R) -> Result<()> {
        self.lua().push_table();
        loop {
            try!(self.read_value(r));   // Key
            if self.lua().type_of(-1) == ValueType::Nil {
                self.lua().pop(1);
                break;
            }
            try!(self.read_value(r));   // Value
            self.lua().set_table(-3);
        }
        Ok(())
    }
}

fn clear_extra<F>(mut lua: LuaState, push_setter: F) -> Result<()>
        where F: FnOnce(&mut LuaState) {
    let base = lua.top_index();
    push_setter(&mut lua);
    lua.push_nil();
    let args = lua.top_index() - base - 1;
    try!(lua.pcall(args, 0, 0));
    Ok(())
}

fn push_setter_and_id<T: ToLua>(lua: &mut LuaState, func: &str, id: T) {
    lua.get_field(REGISTRY_INDEX, func);
    id.to_lua(lua);
}
