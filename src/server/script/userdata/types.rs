use libphysics::{TILE_SIZE, CHUNK_SIZE};

use types::*;

use lua::LuaState;
use script::traits::Userdata;


impl_type_name!(V3);
impl_metatable_key!(V3);
impl_fromlua_copy!(V3);

impl Userdata for V3 {
    fn populate_table(lua: &mut LuaState) {
        lua_table_fns2! {
            lua, -1,

            fn x(v: V3) -> i32 { v.x }
            fn y(v: V3) -> i32 { v.y }
            fn z(v: V3) -> i32 { v.z }

            fn new(x: i32, y: i32, z: i32) -> V3 { V3::new(x, y, z) }

            fn abs(v: V3) -> V3 { v.abs() }
            fn extract(v: V3) -> (i32, i32, i32) { (v.x, v.y, v.z) }

            fn max(v: V3) -> i32 { v.max() }

            fn pixel_to_tile(v: V3) -> V3 {
                v.div_floor(scalar(TILE_SIZE))
            }

            fn tile_to_chunk(v: V3) -> V3 {
                v.div_floor(scalar(CHUNK_SIZE))
            }

            fn reduce(v: V3) -> V2 { v.reduce() }
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


impl_type_name!(V2);
impl_metatable_key!(V2);
impl_fromlua_copy!(V2);

impl Userdata for V2 {
    fn populate_table(lua: &mut LuaState) {
        lua_table_fns2! {
            lua, -1,

            fn x(v: V2) -> i32 { v.x }
            fn y(v: V2) -> i32 { v.y }

            fn new(x: i32, y: i32) -> V2 { V2::new(x, y) }

            fn abs(v: V2) -> V2 { v.abs() }
            fn extract(v: V2) -> (i32, i32) { (v.x, v.y) }

            fn max(v: V2) -> i32 { v.max() }

            fn pixel_to_tile(v: V2) -> V2 {
                v.div_floor(scalar(TILE_SIZE))
            }

            fn tile_to_chunk(v: V2) -> V2 {
                v.div_floor(scalar(CHUNK_SIZE))
            }

            fn extend(v: V2, z: i32) -> V3 { v.extend(z) }
        }
    }

    fn populate_metatable(lua: &mut LuaState) {
        lua_table_fns2! {
            lua, -1,

            fn __add(a: V2, b: V2) -> V2 { a + b }
            fn __sub(a: V2, b: V2) -> V2 { a - b }
            fn __mul(a: V2, b: V2) -> V2 { a * b }
            fn __div(a: V2, b: V2) -> V2 { a / b }
            fn __mod(a: V2, b: V2) -> V2 { a % b }
        }
    }
}
