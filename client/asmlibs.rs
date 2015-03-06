#![crate_name = "asmlibs"]
#![no_std]

#![feature(no_std)]
#![feature(core)]
#![feature(static_assert)]

extern crate core;
extern crate physics;
extern crate graphics;
#[macro_use] extern crate asmrt;

use core::prelude::*;
use core::mem;
use core::raw;
use asmrt::run_callback;
use physics::v3::{V3, scalar};
use physics::{Shape, ShapeSource};
use physics::{CHUNK_SIZE, CHUNK_BITS, CHUNK_MASK};
use graphics::{BlockData, ChunkData, XvData, GeometryBuffer, Sprite};

mod std {
    pub use core::fmt;
    pub use core::marker;
}


pub const LOCAL_SIZE: i32 = 8;
pub const LOCAL_BITS: usize = 3;
pub const LOCAL_MASK: i32 = LOCAL_SIZE - 1;
#[allow(dead_code)] #[static_assert]
static LOCAL_SIZE_BITS: bool = LOCAL_SIZE == 1 << LOCAL_BITS as usize;

pub const REPEAT_SIZE: i32 = 2;
pub const REPEAT_BITS: i32 = 1;
pub const REPEAT_MASK: i32 = REPEAT_SIZE - 1;
#[allow(dead_code)] #[static_assert]
static REPEAT_SIZE_BITS: bool = REPEAT_SIZE == 1 << REPEAT_BITS as usize;


struct AsmJsShapeSource;

impl ShapeSource for AsmJsShapeSource {
    fn get_shape(&self, pos: V3) -> Shape {
        // TODO: Don't hardcode this address!  I'm pretty sure 0x2000 is not even the right address
        // any more.
        const SHAPE_BUFFER: *const Shape = 0x2000 as *const Shape;

        let V3 { x: tile_x, y: tile_y, z: tile_z } = pos & scalar(CHUNK_MASK);
        let V3 { x: chunk_x, y: chunk_y, z: _ } = (pos >> CHUNK_BITS) & scalar(LOCAL_MASK);

        let chunk_idx = chunk_y * LOCAL_SIZE + chunk_x;
        let tile_idx = (tile_z * CHUNK_SIZE + tile_y) * CHUNK_SIZE + tile_x;

        let idx = (chunk_idx << (3 * CHUNK_BITS)) + tile_idx;
        unsafe { *SHAPE_BUFFER.offset(idx as isize) }
    }
}


#[derive(Copy)]
pub struct CollideArgs {
    pub pos: V3,
    pub size: V3,
    pub velocity: V3,
}

#[derive(Copy)]
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


#[export_name = "render"]
pub extern fn render(xv_data: &XvData,
                     x: u16, y: u16, w: u16, h: u16,
                     sprites_ptr: *mut Sprite, sprites_len: i32,
                     draw_terrain_idx: i32,
                     draw_sprite_idx: i32) {
    let draw_terrain = |cx: u16, cy: u16, begin: u16, end: u16| {
        let args = [cx as i32, cy as i32, begin as i32, end as i32];
        run_callback(draw_terrain_idx, args.as_slice());
    };

    let draw_sprite = |id: u16, x: u16, y: u16, w: u16, h: u16| {
        let args = [id as i32, x as i32, y as i32, w as i32, h as i32];
        run_callback(draw_sprite_idx, args.as_slice());
    };

    let sprites = unsafe {
        mem::transmute(raw::Slice {
            data: sprites_ptr as *const Sprite,
            len: sprites_len as usize,
        })
    };

    graphics::render(xv_data, x, y, w, h, sprites, draw_terrain, draw_sprite);
}

#[export_name = "update_xv_data"]
pub extern fn update_xv_data(xv_data: &mut XvData,
                             block_data: &BlockData,
                             chunk_data: &ChunkData,
                             i: u8,
                             j: u8) {
    graphics::update_xv(xv_data, block_data, chunk_data, i, j);
}

#[export_name = "generate_geometry"]
pub extern fn generate_geometry(xv_data: &mut XvData,
                                geom: &mut GeometryBuffer,
                                i: u8, j: u8,
                                vertex_count: &mut i32) {
    *vertex_count = graphics::generate_geometry(xv_data, geom, i, j) as i32;
}


#[repr(C)]
#[derive(Copy, Debug)]
pub struct Sizes {
    xv_data: usize,
    sprite: usize,
    block_data: usize,
    chunk_data: usize,
    geometry_buffer: usize,
}

#[export_name = "get_sizes"]
pub extern fn get_sizes(sizes: &mut Sizes, num_sizes: &mut usize) {
    use core::mem::size_of;

    sizes.xv_data = size_of::<XvData>();
    sizes.sprite = size_of::<Sprite>();
    sizes.block_data = size_of::<BlockData>();
    sizes.chunk_data = size_of::<ChunkData>();
    sizes.geometry_buffer = size_of::<GeometryBuffer>();

    *num_sizes = size_of::<Sizes>() / size_of::<usize>();
}


#[export_name = "test"]
pub extern fn test_wrapper(input: &CollideArgs, output: &mut CollideResult) {
    use core::mem;
    let _ = input;
    let _ = output;
    graphics::update_xv(
        unsafe { mem::transmute(input.pos.x) },
        unsafe { mem::transmute(input.pos.y) },
        unsafe { mem::transmute(input.pos.z) },
        input.size.x as u8, input.size.y as u8);
}
