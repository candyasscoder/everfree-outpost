use std::error;
use std::io;
use std::num::{FromPrimitive, ToPrimitive};
use std::result;
use libc::c_int;

use physics::v3::V2;

use lua::{self, LuaState, ValueType, REGISTRY_INDEX};
use types::*;
use util::StrError;
use world::{World, Client, TerrainChunk, Entity, Structure, Inventory};
use world::object::*;
use world::save::{self, AnyId, ToAnyId, Writer, Reader};

use super::ScriptEngine;
use super::traits::{ToLua, Userdata, metatable_key};
use super::userdata;


pub enum Error {
    Io(io::IoError),
    Str(StrError),
    Lua(String),
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

impl error::FromError<io::IoError> for Error {
    fn from_error(e: io::IoError) -> Error {
        Error::Io(e)
    }
}

impl error::FromError<StrError> for Error {
    fn from_error(e: StrError) -> Error {
        Error::Str(e)
    }
}

impl error::FromError<save::Error> for Error {
    fn from_error(e: save::Error) -> Error {
        match e {
            save::Error::Io(e) => Error::Io(e),
            save::Error::Str(e) => Error::Str(e),
        }
    }
}

impl<'a> error::FromError<(lua::ErrorType, &'a str)> for Error {
    fn from_error((et, s): (lua::ErrorType, &'a str)) -> Error {
        Error::Lua(format!("{:?}: {}", et, s))
    }
}

impl<'a> error::FromError<Error> for save::Error {
    fn from_error(e: Error) -> save::Error {
        match e {
            Error::Io(e) => save::Error::Io(e),
            Error::Str(e) => save::Error::Str(e),
            Error::Lua(_) => save::Error::Str(StrError {
                msg: "error while running lua callback",
            }),
        }
    }
}

pub type Result<T> = result::Result<T, Error>;



#[derive(Copy, PartialEq, Eq, Show, FromPrimitive)]
enum Tag {
    Nil,
    Bool,
    SmallInt,
    LargeInt,
    Float,
    SmallString,
    LargeString,
    Table,

    World,
    Client,
    Entity,
    Structure,
}

pub struct WriteHooks<'a> {
    script: &'a mut ScriptEngine,
}

impl<'a> WriteHooks<'a> {
    pub fn new(script: &'a mut ScriptEngine) -> WriteHooks<'a> {
        WriteHooks {
            script: script,
        }
    }
}

impl<'a> save::WriteHooks for WriteHooks<'a> {
    fn post_write_client<W: Writer>(&mut self,
                                    writer: &mut W,
                                    c: &ObjectRef<Client>) -> save::Result<()> {
        try!(write_extra(self.script.owned_lua.get(), writer, c.id.to_any_id()));
        Ok(())
    }

    fn post_write_terrain_chunk<W: Writer>(&mut self,
                                           _writer: &mut W,
                                           _t: &ObjectRef<TerrainChunk>) -> save::Result<()> {
        // TODO: support terrain_chunk script data
        Ok(())
    }

    fn post_write_entity<W: Writer>(&mut self,
                                    writer: &mut W,
                                    e: &ObjectRef<Entity>) -> save::Result<()> {
        try!(write_extra(self.script.owned_lua.get(), writer, e.id.to_any_id()));
        Ok(())
    }

    fn post_write_structure<W: Writer>(&mut self,
                                       writer: &mut W,
                                       s: &ObjectRef<Structure>) -> save::Result<()> {
        try!(write_extra(self.script.owned_lua.get(), writer, s.id.to_any_id()));
        Ok(())
    }

    fn post_write_inventory<W: Writer>(&mut self,
                                       writer: &mut W,
                                       i: &ObjectRef<Inventory>) -> save::Result<()> {
        try!(write_extra(self.script.owned_lua.get(), writer, i.id.to_any_id()));
        Ok(())
    }
}

fn write_extra<W: Writer>(mut lua: LuaState, w: &mut W, id: AnyId) -> Result<()> {
    try!(get_extra(&mut lua, id));
    try!(write_value(&mut lua, w, -1));
    lua.pop(1);
    Ok(())
}

fn get_extra(lua: &mut LuaState, id: AnyId) -> Result<()> {
    match id {
        AnyId::Client(cid) => {
            lua.get_field(REGISTRY_INDEX, "outpost_callback_get_client_extra");
            cid.unwrap().to_lua(lua);
        },
        AnyId::TerrainChunk(_) => panic!("TerrainChunk script data is not implemented"),
        AnyId::Entity(eid) => {
            lua.get_field(REGISTRY_INDEX, "outpost_callback_get_entity_extra");
            eid.unwrap().to_lua(lua);
        },
        AnyId::Structure(sid) => {
            lua.get_field(REGISTRY_INDEX, "outpost_callback_get_structure_extra");
            sid.unwrap().to_lua(lua);
        },
        AnyId::Inventory(iid) => {
            lua.get_field(REGISTRY_INDEX, "outpost_callback_get_inventory_extra");
            iid.unwrap().to_lua(lua);
        },
    }
    try!(lua.pcall(1, 1, 0));
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
            None
        }
    }

    // Can't use `else if let` here because it would produce overlapping borrows.
    if let Some(_) = get_userdata_opt::<userdata::World>(lua, index) {
        try!(w.write(Tag::World as u8));
        return Ok(());
    }
    if let Some(c) = get_userdata_opt::<userdata::Client>(lua, index) {
        try!(w.write(Tag::Client as u8));
        try!(w.write_id(c.id));
        return Ok(());
    }
    if let Some(e) = get_userdata_opt::<userdata::Entity>(lua, index) {
        try!(w.write(Tag::Entity as u8));
        try!(w.write_id(e.id));
        return Ok(());
    }
    if let Some(s) = get_userdata_opt::<userdata::Structure>(lua, index) {
        try!(w.write(Tag::Structure as u8));
        try!(w.write_id(s.id));
        return Ok(());
    }

    warn!("don't know how to serialize unrecognized userdata");
    Ok(())
}


pub struct ReadHooks<'a> {
    script: &'a mut ScriptEngine,
}

impl<'a> ReadHooks<'a> {
    pub fn new(script: &'a mut ScriptEngine) -> ReadHooks<'a> {
        ReadHooks {
            script: script,
        }
    }
}

impl<'a> save::ReadHooks for ReadHooks<'a> {
    fn post_read_client<R: Reader>(&mut self,
                                   reader: &mut R,
                                   w: &mut World,
                                   cid: ClientId) -> save::Result<()> {
        try!(read_extra(self.script.owned_lua.get(), reader, w, cid.to_any_id()));
        Ok(())
    }

    fn post_read_terrain_chunk<R: Reader>(&mut self,
                                          _reader: &mut R,
                                          _w: &mut World,
                                          _pos: V2) -> save::Result<()> {
        // TODO: support terrain_chunk script data
        Ok(())
    }

    fn post_read_entity<R: Reader>(&mut self,
                                   reader: &mut R,
                                   w: &mut World,
                                   eid: EntityId) -> save::Result<()> {
        try!(read_extra(self.script.owned_lua.get(), reader, w, eid.to_any_id()));
        Ok(())
    }

    fn post_read_structure<R: Reader>(&mut self,
                                      reader: &mut R,
                                      w: &mut World,
                                      sid: StructureId) -> save::Result<()> {
        try!(read_extra(self.script.owned_lua.get(), reader, w, sid.to_any_id()));
        Ok(())
    }

    fn post_read_inventory<R: Reader>(&mut self,
                                      reader: &mut R,
                                      w: &mut World,
                                      iid: InventoryId) -> save::Result<()> {
        try!(read_extra(self.script.owned_lua.get(), reader, w, iid.to_any_id()));
        Ok(())
    }

    fn cleanup_client(&mut self, _w: &mut World, cid: ClientId) -> save::Result<()> {
        try!(clear_extra(self.script.owned_lua.get(), cid.to_any_id()));
        Ok(())
    }

    fn cleanup_terrain_chunk(&mut self, _w: &mut World, _pos: V2) -> save::Result<()> {
        // TODO: support terrain_chunk script data
        Ok(())
    }

    fn cleanup_entity(&mut self, _w: &mut World, eid: EntityId) -> save::Result<()> {
        try!(clear_extra(self.script.owned_lua.get(), eid.to_any_id()));
        Ok(())
    }

    fn cleanup_structure(&mut self, _w: &mut World, sid: StructureId) -> save::Result<()> {
        try!(clear_extra(self.script.owned_lua.get(), sid.to_any_id()));
        Ok(())
    }

    fn cleanup_inventory(&mut self, _w: &mut World, iid: InventoryId) -> save::Result<()> {
        try!(clear_extra(self.script.owned_lua.get(), iid.to_any_id()));
        Ok(())
    }
}

fn read_extra<R: Reader>(mut lua: LuaState, r: &mut R, world: &mut World, id: AnyId) -> Result<()> {
    push_setter_and_id(&mut lua, id);
    try!(read_value(&mut lua, r, world));
    try!(lua.pcall(2, 0, 0));
    Ok(())
}

fn clear_extra(mut lua: LuaState, id: AnyId) -> Result<()> {
    push_setter_and_id(&mut lua, id);
    lua.push_nil();
    try!(lua.pcall(2, 0, 0));
    Ok(())
}

fn push_setter_and_id(lua: &mut LuaState, id: AnyId) {
    match id {
        AnyId::Client(cid) => {
            lua.get_field(REGISTRY_INDEX, "outpost_callback_set_client_extra");
            cid.unwrap().to_lua(lua);
        },
        AnyId::TerrainChunk(_) => panic!("TerrainChunk script data is not implemented"),
        AnyId::Entity(eid) => {
            lua.get_field(REGISTRY_INDEX, "outpost_callback_set_entity_extra");
            eid.unwrap().to_lua(lua);
        },
        AnyId::Structure(sid) => {
            lua.get_field(REGISTRY_INDEX, "outpost_callback_set_structure_extra");
            sid.unwrap().to_lua(lua);
        },
        AnyId::Inventory(iid) => {
            lua.get_field(REGISTRY_INDEX, "outpost_callback_set_inventory_extra");
            iid.unwrap().to_lua(lua);
        },
    }
}

fn read_value<R: Reader>(lua: &mut LuaState, r: &mut R, world: &mut World) -> Result<()> {
    let (tag, a, b): (u8, u8, u16) = try!(r.read());
    let tag = unwrap!(FromPrimitive::from_u8(tag));
    match tag {
        Tag::Nil => {
            lua.push_nil();
        },
        Tag::Bool => {
            lua.push_boolean(a != 0);
        },
        Tag::SmallInt => {
            lua.push_integer(b as isize);
        },
        Tag::LargeInt => {
            let i: i32 = try!(r.read());
            lua.push_integer(i as isize);
        },
        Tag::Float => {
            let f: f64 = try!(r.read());
            lua.push_number(f);
        },
        Tag::SmallString => {
            let s = try!(r.read_str_bytes(b as usize));
            lua.push_string(&*s);
        },
        Tag::LargeString => {
            let s = try!(r.read_str());
            lua.push_string(&*s);
        },
        Tag::Table => {
            try!(read_table(lua, r, world));
        },

        Tag::World => {
            let w = userdata::World;
            w.to_lua(lua);
        },
        Tag::Client => {
            let cid = try!(r.read_id(world));
            let c = userdata::Client { id: cid };
            c.to_lua(lua);
        },
        Tag::Entity => {
            let eid = try!(r.read_id(world));
            let e = userdata::Entity { id: eid };
            e.to_lua(lua);
        },
        Tag::Structure => {
            let sid = try!(r.read_id(world));
            let s = userdata::Structure { id: sid };
            s.to_lua(lua);
        },
    }
    Ok(())
}

fn read_table<R: Reader>(lua: &mut LuaState, r: &mut R, world: &mut World) -> Result<()> {
    lua.push_table();
    loop {
        try!(read_value(lua, r, world));    // Key
        if lua.type_of(-1) == ValueType::Nil {
            lua.pop(1);
            break;
        }
        try!(read_value(lua, r, world));    // Value
        lua.set_table(-3);
    }
    Ok(())
}
