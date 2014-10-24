#![crate_name = "asmlibs"]
#![no_std]

extern crate core;
extern crate physics;

use core::ptr::RawPtr;
use physics::v3::V3;
use physics::{Shape, Empty, ShapeSource, CHUNK_SIZE};


struct AsmJsShapeSource;

impl ShapeSource for AsmJsShapeSource {
    fn get_shape(&self, pos: V3) -> Shape {
        const SHAPE_BUFFER: *const Shape = 0x2000 as *const Shape;

        let V3 { x, y, z } = pos;
        if x < 0 || x >= CHUNK_SIZE || y < 0 || y >= CHUNK_SIZE || z < 0 || z >= CHUNK_SIZE {
            return Empty;
        }

        let index = ((z) * CHUNK_SIZE + y) * CHUNK_SIZE + x;
        unsafe { *SHAPE_BUFFER.offset(index as int) }
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
