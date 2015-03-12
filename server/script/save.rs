use std::error;
use std::fmt;
use std::num::FromPrimitive;
use std::old_io;
use std::result;
use libc::c_int;

use lua::{self, LuaState, ValueType, REGISTRY_INDEX};
use types::*;
use util::Convert;
use util::Stable;
use util::StrError;
use world::{World, Client, TerrainChunk, Entity, Structure, Inventory};
use world::object::*;
use world::save::{self, Writer, Reader};

use super::ScriptEngine;
use super::traits::{ToLua, Userdata, metatable_key};
use super::userdata;


pub enum Error {
    Io(old_io::IoError),
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

impl error::FromError<old_io::IoError> for Error {
    fn from_error(e: old_io::IoError) -> Error {
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



#[derive(Copy, PartialEq, Eq, Debug, FromPrimitive)]
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
    Inventory,

    StableClient,
    StableEntity,
    StableStructure,
    StableInventory,

    V3,
}


// NB: This typedef is the same as engine::glue::SaveWriteHooks
engine_part_typedef!(pub WriteHooks(script));

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

    fn post_write_terrain_chunk<W: Writer>(&mut self,
                                           _writer: &mut W,
                                           _t: &ObjectRef<TerrainChunk>) -> save::Result<()> {
        // TODO: support terrain_chunk script data
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

    fn post_write_structure<W: Writer>(&mut self,
                                       writer: &mut W,
                                       s: &ObjectRef<Structure>) -> save::Result<()> {
        try!(write_extra(self.script_mut().owned_lua.get(), writer, |lua| {
            call_get_extra(lua, "outpost_callback_get_structure_extra", s.id().unwrap())
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
    if let Some(i) = get_userdata_opt::<userdata::Inventory>(lua, index) {
        try!(w.write(Tag::Inventory as u8));
        try!(w.write_id(i.id));
        return Ok(());
    }

    if let Some(c) = get_userdata_opt::<userdata::StableClient>(lua, index) {
        try!(w.write(Tag::StableClient as u8));
        try!(w.write(c.id.val));
        return Ok(());
    }
    if let Some(e) = get_userdata_opt::<userdata::StableEntity>(lua, index) {
        try!(w.write(Tag::StableEntity as u8));
        try!(w.write(e.id.val));
        return Ok(());
    }
    if let Some(s) = get_userdata_opt::<userdata::StableStructure>(lua, index) {
        try!(w.write(Tag::StableStructure as u8));
        try!(w.write(s.id.val));
        return Ok(());
    }
    if let Some(i) = get_userdata_opt::<userdata::StableInventory>(lua, index) {
        try!(w.write(Tag::StableInventory as u8));
        try!(w.write(i.id.val));
        return Ok(());
    }

    if let Some(v) = get_userdata_opt::<V3>(lua, index) {
        try!(w.write(Tag::V3 as u8));
        try!(w.write((v.x, v.y, v.z)));
        return Ok(());
    }

    warn!("don't know how to serialize unrecognized userdata");
    lua.pop(1);
    Ok(())
}


pub struct ReadHooks<'a> {
    script: &'a mut ScriptEngine,
}

impl<'a> save::ReadHooks for ReadHooks<'a> {}

/*
impl<'a> ReadHooks<'a> {
    pub fn new(script: &'a mut ScriptEngine) -> ReadHooks<'a> {
        ReadHooks {
            script: script,
        }
    }
}

impl<'a, W: WorldMut> save::ReadHooks<W> for ReadHooks<'a> {
    fn post_read_world<R: Reader>(&mut self,
                                  reader: &mut R,
                                  w: &mut W) -> save::Result<()> {
        try!(read_extra(self.script.owned_lua.get(), reader, w, |lua| {
            lua.get_field(REGISTRY_INDEX, "outpost_callback_set_world_extra");
        }));
        Ok(())
    }

    fn post_read_client<R: Reader>(&mut self,
                                   reader: &mut R,
                                   w: &mut W,
                                   cid: ClientId) -> save::Result<()> {
        try!(read_extra(self.script.owned_lua.get(), reader, w, |lua| {
            push_setter_and_id(lua, "outpost_callback_set_client_extra", cid.unwrap())
        }));
        Ok(())
    }

    fn post_read_terrain_chunk<R: Reader>(&mut self,
                                          _reader: &mut R,
                                          _w: &mut W,
                                          _pos: V2) -> save::Result<()> {
        // TODO: support terrain_chunk script data
        Ok(())
    }

    fn post_read_entity<R: Reader>(&mut self,
                                   reader: &mut R,
                                   w: &mut W,
                                   eid: EntityId) -> save::Result<()> {
        try!(read_extra(self.script.owned_lua.get(), reader, w, |lua| {
            push_setter_and_id(lua, "outpost_callback_set_entity_extra", eid.unwrap())
        }));
        Ok(())
    }

    fn post_read_structure<R: Reader>(&mut self,
                                      reader: &mut R,
                                      w: &mut W,
                                      sid: StructureId) -> save::Result<()> {
        try!(read_extra(self.script.owned_lua.get(), reader, w, |lua| {
            push_setter_and_id(lua, "outpost_callback_set_structure_extra", sid.unwrap())
        }));
        Ok(())
    }

    fn post_read_inventory<R: Reader>(&mut self,
                                      reader: &mut R,
                                      w: &mut W,
                                      iid: InventoryId) -> save::Result<()> {
        try!(read_extra(self.script.owned_lua.get(), reader, w, |lua| {
            push_setter_and_id(lua, "outpost_callback_set_inventory_extra", iid.unwrap())
        }));
        Ok(())
    }

    fn cleanup_world(&mut self, _w: &mut W) -> save::Result<()> {
        try!(clear_extra(self.script.owned_lua.get(), |lua| {
            lua.get_field(REGISTRY_INDEX, "outpost_callback_set_world_extra");
        }));
        Ok(())
    }

    fn cleanup_client(&mut self, _w: &mut W, cid: ClientId) -> save::Result<()> {
        try!(clear_extra(self.script.owned_lua.get(), |lua| {
            push_setter_and_id(lua, "outpost_callback_set_client_extra", cid.unwrap())
        }));
        Ok(())
    }

    fn cleanup_terrain_chunk(&mut self, _w: &mut W, _pos: V2) -> save::Result<()> {
        // TODO: support terrain_chunk script data
        Ok(())
    }

    fn cleanup_entity(&mut self, _w: &mut W, eid: EntityId) -> save::Result<()> {
        try!(clear_extra(self.script.owned_lua.get(), |lua| {
            push_setter_and_id(lua, "outpost_callback_set_entity_extra", eid.unwrap())
        }));
        Ok(())
    }

    fn cleanup_structure(&mut self, _w: &mut W, sid: StructureId) -> save::Result<()> {
        try!(clear_extra(self.script.owned_lua.get(), |lua| {
            push_setter_and_id(lua, "outpost_callback_set_structure_extra", sid.unwrap())
        }));
        Ok(())
    }

    fn cleanup_inventory(&mut self, _w: &mut W, iid: InventoryId) -> save::Result<()> {
        try!(clear_extra(self.script.owned_lua.get(), |lua| {
            push_setter_and_id(lua, "outpost_callback_set_inventory_extra", iid.unwrap())
        }));
        Ok(())
    }
}

fn read_extra<R: Reader, F, W: WorldMut>(mut lua: LuaState,
                                         r: &mut R,
                                         world: &mut W,
                                         push_setter: F) -> Result<()>
        where F: FnOnce(&mut LuaState) {
    let base = lua.top_index();
    push_setter(&mut lua);
    try!(read_value(&mut lua, r, world));
    let args = lua.top_index() - base - 1;
    try!(lua.pcall(args, 0, 0));
    Ok(())
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

fn read_value<R: Reader, W: WorldMut>(lua: &mut LuaState, r: &mut R, world: &mut W) -> Result<()> {
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
        Tag::Inventory => {
            let iid = try!(r.read_id(world));
            let i = userdata::Inventory { id: iid };
            i.to_lua(lua);
        },

        Tag::StableClient => {
            let cid = try!(r.read());
            let c = userdata::StableClient { id: Stable::new(cid) };
            c.to_lua(lua);
        },
        Tag::StableEntity => {
            let eid = try!(r.read());
            let e = userdata::StableEntity { id: Stable::new(eid) };
            e.to_lua(lua);
        },
        Tag::StableStructure => {
            let sid = try!(r.read());
            let s = userdata::StableStructure { id: Stable::new(sid) };
            s.to_lua(lua);
        },
        Tag::StableInventory => {
            let iid = try!(r.read());
            let i = userdata::StableInventory { id: Stable::new(iid) };
            i.to_lua(lua);
        },

        Tag::V3 => {
            let (x, y, z) = try!(r.read());
            V3::new(x, y, z).to_lua(lua);
        },
    }
    Ok(())
}

fn read_table<R: Reader, W: WorldMut>(lua: &mut LuaState,
                                      r: &mut R,
                                      world: &mut W) -> Result<()> {
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
*/
