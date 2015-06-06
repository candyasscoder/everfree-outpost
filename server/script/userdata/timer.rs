use std::ptr;
use libc::c_int;

use types::*;
use util::StrResult;

use engine;
use lua::{LuaState, ValueType};
use messages;
use script::traits::{TypeName, MetatableKey, FromLua, ToLua, Userdata, is_userdata};
use script::userdata::OptWrapper;
use timer;


enum TimeOrNumber {
    Time(Time),
    Number(i32),
}

impl<'a> FromLua<'a> for TimeOrNumber {
    unsafe fn check(lua: &mut LuaState, index: c_int, func: &'static str) {
        if lua.type_of(index) != ValueType::Number && !is_userdata::<TimeU>(lua, index) {
            type_error!(lua, index, func, "number or Time");
        }
    }

    unsafe fn from_lua(lua: &'a LuaState, index: c_int) -> TimeOrNumber {
        if lua.type_of(index) == ValueType::Number {
            TimeOrNumber::Number(FromLua::from_lua(lua, index))
        } else {
            TimeOrNumber::Time(<TimeU as FromLua>::from_lua(lua, index).t)
        }
    }
}

impl ToLua for TimeOrNumber {
    fn to_lua(self, lua: &mut LuaState) {
        match self {
            TimeOrNumber::Time(t) => (TimeU { t: t }).to_lua(lua),
            TimeOrNumber::Number(n) => n.to_lua(lua),
        }
    }
}


#[derive(Clone, Copy)]
pub struct TimeU {
    pub t: Time,
}

impl TypeName for TimeU {
    fn type_name() -> &'static str { "Time" }
}

impl MetatableKey for TimeU {
    fn metatable_key() -> &'static str { "outpost_metatable_Time" }
}

impl_fromlua_copy!(TimeU);

impl Userdata for TimeU {
    fn populate_table(lua: &mut LuaState) {
        lua_table_fns2! {
            lua, -1,

            fn now(!partial ctx: &engine::Engine, _x: ()) -> TimeU {
                TimeU { t: ctx.now() }
            }
        }
    }

    fn populate_metatable(lua: &mut LuaState) {
        lua_table_fns2! {
            lua, -1,

            fn __add(this: TimeU, other: i32) -> TimeU {
                // Use wrapping ops for safety, so that scripts can't crash the server.
                TimeU { t: this.t.wrapping_add(other as Time) }
            }

            fn __sub(this: TimeU, other: TimeOrNumber) -> TimeOrNumber {
                match other {
                    TimeOrNumber::Time(t) =>
                        TimeOrNumber::Number(this.t.wrapping_sub(t) as i32),
                    TimeOrNumber::Number(n) =>
                        TimeOrNumber::Time(this.t.wrapping_sub(n as Time)),
                }
            }

            fn __eq(this: TimeU, other: TimeU) -> bool {
                this.t == other.t
            }

            fn __lt(this: TimeU, other: TimeU) -> bool {
                this.t < other.t
            }

            fn __le(this: TimeU, other: TimeU) -> bool {
                this.t <= other.t
            }

            fn __tostring(this: TimeU) -> String {
                format!("{}", this.t)
            }
        }
    }
}


pub type Timer = OptWrapper<timer::Cookie>;

impl_type_name!(Timer);
impl_metatable_key!(Timer);

impl Userdata for Timer {
    fn populate_table(lua: &mut LuaState) {
        lua_table_fns2! {
            lua, -1,

            fn schedule(!partial ctx: &mut messages::Messages,
                        when: TimeU,
                        id: u32) -> Timer {
                Timer::new(ctx.schedule_script_timeout(when.t, id))
            }

            fn cancel(!partial ctx: &mut messages::Messages,
                      this: &Timer) -> StrResult<()> {
                let cookie = unwrap!(this.take());
                ctx.cancel(cookie);
                Ok(())
            }
        }
    }

    fn populate_metatable(lua: &mut LuaState) {
        lua_table_fns2! {
            lua, -1,

            fn __gc(x: &Timer) -> () {
                unsafe { ptr::read(x as *const Timer) };
            }
        }
    }
}
