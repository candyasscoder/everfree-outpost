use std::collections::HashMap;
use libc::c_int;

use types::*;
use util::StrResult;

use lua::{LuaState, ValueType};
use msg;
use script::traits::{FromLua, ToLua, Userdata};
use script::userdata::OptWrapper;


impl<'a> FromLua<'a> for msg::SimpleArg {
    unsafe fn check(lua: &mut LuaState, index: c_int, func: &'static str) {
        let ty = lua.type_of(index);
        if ty != ValueType::Number && ty != ValueType::String {
            type_error!(lua, index, func, "number or string");
        }
    }

    unsafe fn from_lua(lua: &'a LuaState, index: c_int) -> msg::SimpleArg {
        let ty = lua.type_of(index);
        match ty {
            ValueType::Number => msg::SimpleArg::Int(FromLua::from_lua(lua, index)),
            ValueType::String => msg::SimpleArg::Str(FromLua::from_lua(lua, index)),
            _ => unreachable!("expected ValueType::Number or ValueType::String"),
        }
    }
}

impl ToLua for msg::SimpleArg {
    fn to_lua(self, lua: &mut LuaState) {
        match self {
            msg::SimpleArg::Int(i) => i.to_lua(lua),
            msg::SimpleArg::Str(s) => s.to_lua(lua),
        }
    }
}



pub type ExtraArg = OptWrapper<msg::ExtraArg>;

impl_type_name!(ExtraArg);
impl_metatable_key!(ExtraArg);

impl Userdata for ExtraArg {
    fn populate_table(lua: &mut LuaState) {
        lua_table_fns2! {
            lua, -1,

            fn int(i: i32) -> ExtraArg {
                ExtraArg::new(msg::ExtraArg::Int(i))
            }

            fn str(s: String) -> ExtraArg {
                ExtraArg::new(msg::ExtraArg::Str(s))
            }

            fn list() -> ExtraArg {
                ExtraArg::new(msg::ExtraArg::List(Vec::new()))
            }

            fn map() -> ExtraArg {
                ExtraArg::new(msg::ExtraArg::Map(HashMap::new()))
            }


            fn get_type(a: &ExtraArg) -> StrResult<&'static str> {
                a.open(|a| {
                    match *a {
                        msg::ExtraArg::Int(_) => "int",
                        msg::ExtraArg::Str(_) => "str",
                        msg::ExtraArg::List(_) => "list",
                        msg::ExtraArg::Map(_) => "map",
                    }
                })
            }


            fn as_int(a: &ExtraArg) -> StrResult<i32> {
                flatten(a.open(|a| {
                    match *a {
                        msg::ExtraArg::Int(i) => Ok(i),
                        _ => fail!("expected ExtraArg::Int"),
                    }
                }))
            }

            fn as_str(a: &ExtraArg) -> StrResult<String> {
                flatten(a.open(|a| {
                    match *a {
                        msg::ExtraArg::Str(ref s) => Ok(s.clone()),
                        _ => fail!("expected ExtraArg::Str"),
                    }
                }))
            }


            fn push(a: &ExtraArg, b: &ExtraArg) -> StrResult<()> {
                let v = unwrap!(b.take());
                flatten(a.open(|a| {
                    match *a {
                        msg::ExtraArg::List(ref mut l) => { l.push(v); Ok(()) },
                        _ => fail!("expected ExtraArg::List as first argument"),
                    }
                }))
                // TODO: should probably replace `v` into `b` on failure.
            }

            fn pop(a: &ExtraArg) -> StrResult<Option<ExtraArg>> {
                flatten(a.open(|a| {
                    match *a {
                        msg::ExtraArg::List(ref mut l) => Ok(l.pop().map(|a| ExtraArg::new(a))),
                        _ => fail!("expected ExtraArg::List"),
                    }
                }))
            }

            fn len(a: &ExtraArg) -> StrResult<i32> {
                flatten(a.open(|a| {
                    match *a {
                        msg::ExtraArg::List(ref l) => Ok(l.len() as i32),
                        msg::ExtraArg::Map(ref m) => Ok(m.len() as i32),
                        _ => fail!("expected ExtraArg::List or ExtraArg::Map"),
                    }
                }))
            }

            fn get(a: &ExtraArg, k: msg::SimpleArg) -> StrResult<ExtraArg> {
                flatten(a.open(|a| {
                    match *a {
                        msg::ExtraArg::List(ref l) =>
                            if let msg::SimpleArg::Int(i) = k {
                                let i = i as usize;
                                if i < l.len() {
                                    Ok(ExtraArg::new(l[i].clone()))
                                } else {
                                    fail!("index out of bounds");
                                }
                            } else {
                                fail!("expected number as second argument");
                            },
                        msg::ExtraArg::Map(ref m) =>
                            if let Some(x) = m.get(&k) {
                                Ok(ExtraArg::new(x.clone()))
                            } else {
                                fail!("key not present in map");
                            },
                        _ => fail!("expected ExtraArg::List or ExtraArg::Map as first argument"),
                    }
                }))
            }

            fn set(a: &ExtraArg, k: msg::SimpleArg, v: &ExtraArg) -> StrResult<()> {
                let v = unwrap!(v.take());
                flatten(a.open(|a| {
                    match *a {
                        msg::ExtraArg::List(ref mut l) =>
                            if let msg::SimpleArg::Int(i) = k {
                                let i = i as usize;
                                if i < l.len() {
                                    l[i] = v;
                                    Ok(())
                                } else {
                                    fail!("index out of bounds");
                                }
                            } else {
                                fail!("expected number as second argument");
                            },
                        msg::ExtraArg::Map(ref mut m) => {
                            m.insert(k, v);
                            Ok(())
                        },
                        _ => fail!("expected ExtraArg::List or ExtraArg::Map as first argument"),
                    }
                }))
            }
        }
    }

    fn populate_metatable(lua: &mut LuaState) {
        lua_table_fns2! {
            lua, -1,

            fn __gc(x: &ExtraArg) -> () {
                x.take();
            }
        }
    }
}

fn flatten<T, E>(x: Result<Result<T, E>, E>) -> Result<T, E> {
    match x {
        Err(e) => Err(e),
        Ok(Err(e)) => Err(e),
        Ok(Ok(x)) => Ok(x),
    }
}
