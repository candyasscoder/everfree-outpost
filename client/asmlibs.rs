#![crate_name = "asmlibs"]
#![no_std]
#![feature(phase)]

extern crate core;
extern crate physics;
#[phase(plugin, link)] extern crate asmrt;

use core::ptr::RawPtr;
use physics::v3::{V3, scalar};
use physics::{Shape, ShapeSource};
use physics::{CHUNK_SIZE, CHUNK_BITS, CHUNK_MASK};

mod std {
    pub use core::fmt;
}


pub const LOCAL_SIZE: i32 = 8;
pub const LOCAL_BITS: uint = 3;
pub const LOCAL_MASK: i32 = LOCAL_SIZE - 1;
#[allow(dead_code)] #[static_assert]
static LOCAL_SIZE_BITS: bool = LOCAL_SIZE == 1 << LOCAL_BITS as uint;

pub const REPEAT_SIZE: i32 = 2;
pub const REPEAT_BITS: i32 = 1;
pub const REPEAT_MASK: i32 = REPEAT_SIZE - 1;
#[allow(dead_code)] #[static_assert]
static REPEAT_SIZE_BITS: bool = REPEAT_SIZE == 1 << REPEAT_BITS as uint;

struct AsmJsShapeSource;

impl ShapeSource for AsmJsShapeSource {
    fn get_shape(&self, pos: V3) -> Shape {
        const SHAPE_BUFFER: *const Shape = 0x2000 as *const Shape;

        let V3 { x: tile_x, y: tile_y, z: tile_z } = pos & scalar(CHUNK_MASK);
        let V3 { x: chunk_x, y: chunk_y, z: _ } = (pos >> CHUNK_BITS) & scalar(LOCAL_MASK);

        let chunk_idx = chunk_y * LOCAL_SIZE + chunk_x;
        let tile_idx = (tile_z * CHUNK_SIZE + tile_y) * CHUNK_SIZE + tile_x;

        let idx = (chunk_idx << (3 * CHUNK_BITS)) + tile_idx;
        unsafe { *SHAPE_BUFFER.offset(idx as int) }
    }
}


pub struct CollideArgs {
    pub pos: V3,
    pub size: V3,
    pub velocity: V3,
}

pub struct CollideResult {
    pub pos: V3,
    pub time: i32,
}

#[export_name = "collide"]
pub extern fn collide_wrapper(input: &CollideArgs, output: &mut CollideResult) {
    let (pos, time) = physics::collide(&AsmJsShapeSource,
                                       input.pos, input.size, input.velocity);
    output.pos = pos;
    output.time = time;
}

#[export_name = "test"]
pub extern fn test_wrapper(input: &CollideArgs, output: &mut CollideResult) {
    output.time = AsmJsShapeSource.get_shape(input.pos) as i32;
}
