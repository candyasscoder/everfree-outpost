use physics::{TILE_SIZE, CHUNK_SIZE};

use types::*;

use lua::LuaState;
use script::traits::Userdata;


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
