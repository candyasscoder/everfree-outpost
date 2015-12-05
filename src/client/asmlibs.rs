#![crate_name = "asmlibs"]
#![no_std]

#![feature(no_std)]
#![feature(core, core_prelude, core_slice_ext)]
#![feature(raw)]

extern crate core;
extern crate physics;
extern crate graphics;
#[macro_use] extern crate asmrt;

use core::prelude::*;
use core::mem;
use core::raw;
use core::slice;
use physics::v3::{V3, V2, scalar, Region};
use physics::{Shape, ShapeSource};
use physics::{CHUNK_SIZE, CHUNK_BITS, CHUNK_MASK, TILE_BITS};

use graphics::lights;
use graphics::structures;
use graphics::terrain;
use graphics::types as gfx_types;

mod std {
    pub use core::fmt;
    pub use core::marker;
}


pub const LOCAL_BITS: usize = 3;
pub const LOCAL_SIZE: i32 = 1 << LOCAL_BITS;    // 8
pub const LOCAL_MASK: i32 = LOCAL_SIZE - 1;

pub const REPEAT_BITS: i32 = 1;
pub const REPEAT_SIZE: i32 = 1 << REPEAT_BITS;  // 2
pub const REPEAT_MASK: i32 = REPEAT_SIZE - 1;

pub const NUM_LAYERS: usize = 3;


// Physics

pub type ShapeChunk = [Shape; 1 << (3 * CHUNK_BITS)];

pub struct ShapeLayers {
    base: ShapeChunk,
    layers: [ShapeChunk; NUM_LAYERS],
    merged: ShapeChunk,
}

impl ShapeLayers {
    fn refresh(&mut self, bounds: Region) {
        let chunk_bounds = Region::new(scalar(0), scalar(CHUNK_SIZE));

        for p in bounds.intersect(chunk_bounds).points() {
            let idx = chunk_bounds.index(p);
            self.merged[idx] = self.base[idx];

            for layer in self.layers.iter() {
                if shape_overrides(self.merged[idx], layer[idx]) {
                    self.merged[idx] = layer[idx];
                }
            }
        }
    }
}

fn shape_overrides(old: Shape, new: Shape) -> bool {
    match (old, new) {
        (Shape::Empty, _) => true,

        (Shape::Floor, Shape::Empty) => false,
        (Shape::Floor, _) => true,

        (Shape::Solid, _) => false,

        _ => false,
    }
}

struct AsmJsShapeSource<'a> {
    layers: &'a [ShapeLayers; 1 << (2 * LOCAL_BITS)],
}

impl<'a> ShapeSource for AsmJsShapeSource<'a> {
    fn get_shape(&self, pos: V3) -> Shape {
        if pos.z < 0 || pos.z >= CHUNK_SIZE {
            return Shape::Empty;
        }

        let V3 { x: tile_x, y: tile_y, z: tile_z } = pos & V3::new(CHUNK_MASK, CHUNK_MASK, -1);
        let V3 { x: chunk_x, y: chunk_y, z: _ } = (pos >> CHUNK_BITS) & scalar(LOCAL_MASK);

        let chunk_idx = chunk_y * LOCAL_SIZE + chunk_x;
        let tile_idx = (tile_z * CHUNK_SIZE + tile_y) * CHUNK_SIZE + tile_x;

        let shape = self.layers[chunk_idx as usize].merged[tile_idx as usize];
        shape
    }
}


#[derive(Clone, Copy)]
pub struct CollideArgs {
    pub pos: V3,
    pub size: V3,
    pub velocity: V3,
}

#[derive(Clone, Copy)]
pub struct CollideResult {
    pub pos: V3,
    pub time: i32,
}

#[export_name = "collide"]
pub extern fn collide_wrapper(layers: &[ShapeLayers; 1 << (2 * LOCAL_BITS)],
                              input: &CollideArgs,
                              output: &mut CollideResult) {
    let (pos, time) = physics::collide(&AsmJsShapeSource { layers: layers },
                                       input.pos, input.size, input.velocity);
    output.pos = pos;
    output.time = time;
}

#[export_name = "set_region_shape"]
pub extern fn set_region_shape(layers: &mut [ShapeLayers; 1 << (2 * LOCAL_BITS)],
                               bounds: &Region,
                               layer: usize,
                               shape_data: *const Shape,
                               shape_len: usize) {
    let shape: &[Shape] = unsafe {
        mem::transmute(raw::Slice {
            data: shape_data,
            len: shape_len,
        })
    };

    let chunk_bounds = Region::new(scalar(0), scalar(CHUNK_SIZE));
    for p in bounds.points() {
        // div_floor requires an extra LLVM intrinsic.
        let cpos = p.reduce() >> CHUNK_BITS;
        let masked_cpos = cpos & scalar(LOCAL_MASK);
        let cidx = masked_cpos.y * LOCAL_SIZE + masked_cpos.x;

        let offset = p & scalar(CHUNK_MASK);
        let out_idx = chunk_bounds.index(offset);
        let in_idx = bounds.index(p);

        layers[cidx as usize].layers[layer][out_idx] = shape[in_idx];
    }
}

#[export_name = "refresh_shape_cache"]
pub extern fn refresh_shape_cache(layers: &mut [ShapeLayers; 1 << (2 * LOCAL_BITS)],
                                  bounds: &Region) {
    let chunk_bounds = bounds.reduce().div_round(CHUNK_SIZE);

    for cpos in chunk_bounds.points() {
        let masked_cpos = cpos & scalar(LOCAL_MASK);
        let cidx = masked_cpos.y * LOCAL_SIZE + masked_cpos.x;

        let base = cpos.extend(0) * scalar(CHUNK_SIZE);
        layers[cidx as usize].refresh(*bounds - base);
    }
}

#[export_name = "find_ceiling"]
pub extern fn find_ceiling(layers: &[ShapeLayers; 1 << (2 * LOCAL_BITS)],
                           pos: &V3) -> i32 {
    let tpos = *pos >> TILE_BITS;

    let cx = tpos.x / CHUNK_SIZE % LOCAL_SIZE;
    let cy = tpos.y / CHUNK_SIZE % LOCAL_SIZE;
    let idx = (cy * LOCAL_SIZE + cx) as usize;

    let x = tpos.x % CHUNK_SIZE;
    let y = tpos.y % CHUNK_SIZE;
    let mut z = tpos.z + 1;

    let chunk = &layers[idx];
    while z < 16 {
        let tile_idx = x + CHUNK_SIZE * (y + CHUNK_SIZE * (z));
        if chunk.merged[tile_idx as usize] != Shape::Empty {
            break;
        }
        z += 1;
    }
    z
}


// Graphics

unsafe fn make_slice<T>(ptr: *const T, byte_len: usize) -> &'static [T] {
    slice::from_raw_parts(ptr, byte_len / mem::size_of::<T>())
}

unsafe fn make_slice_mut<T>(ptr: *mut T, byte_len: usize) -> &'static mut [T] {
    slice::from_raw_parts_mut(ptr, byte_len / mem::size_of::<T>())
}

pub struct GeometryResult {
    vertex_count: usize,
    more: u8,
}


#[export_name = "terrain_geom_init"]
pub unsafe extern fn terrain_geom_init(geom: &mut terrain::GeomGen<'static>,
                                       block_data_ptr: *const gfx_types::BlockData,
                                       block_data_byte_len: usize,
                                       local_chunks: &'static gfx_types::LocalChunks) {
    let block_data = make_slice(block_data_ptr, block_data_byte_len);
    geom.init(block_data, local_chunks);
}

#[export_name = "terrain_geom_reset"]
pub extern fn terrain_geom_reset(geom: &mut terrain::GeomGen,
                                 cx: i32,
                                 cy: i32) {
    geom.reset(V2::new(cx, cy));
}

#[export_name = "terrain_geom_generate"]
pub unsafe extern fn terrain_geom_generate(geom: &mut terrain::GeomGen,
                                           buf_ptr: *mut terrain::Vertex,
                                           buf_byte_len: usize,
                                           result: &mut GeometryResult) {
    let buf = make_slice_mut(buf_ptr, buf_byte_len);

    let mut idx = 0;
    let more = geom.generate(buf, &mut idx);

    result.vertex_count = idx;
    result.more = more as u8;
}


#[export_name = "structure_buffer_init"]
pub unsafe extern fn structure_buffer_init(buf: &mut structures::Buffer<'static>,
                                           storage_ptr: *mut structures::Structure,
                                           storage_byte_len: usize) {
    let storage = make_slice_mut(storage_ptr, storage_byte_len);
    buf.init(storage);
}

#[export_name = "structure_buffer_insert"]
pub extern fn structure_buffer_insert(buf: &mut structures::Buffer,
                                      external_id: u32,
                                      pos_x: u8,
                                      pos_y: u8,
                                      pos_z: u8,
                                      template_id: u32,
                                      oneshot_start: u16) -> usize {
    if let Some(idx) = buf.insert(external_id,
                                  (pos_x, pos_y, pos_z),
                                  template_id) {
        buf[idx].oneshot_start = oneshot_start;
        idx
    } else {
        (-1_isize) as usize
    }
}

#[export_name = "structure_buffer_remove"]
pub extern fn structure_buffer_remove(buf: &mut structures::Buffer,
                                      idx: usize) -> u32 {
    buf.remove(idx)
}


#[export_name = "structure_geom_init"]
pub unsafe extern fn structure_geom_init(geom: &mut structures::GeomGen<'static>,
                                         buffer: &'static structures::Buffer<'static>,
                                         templates_ptr: *const gfx_types::StructureTemplate,
                                         templates_byte_len: usize,
                                         parts_ptr: *const gfx_types::TemplatePart,
                                         parts_byte_len: usize,
                                         verts_ptr: *const gfx_types::TemplateVertex,
                                         verts_byte_len: usize) {
    let templates = make_slice(templates_ptr, templates_byte_len);
    let parts = make_slice(parts_ptr, parts_byte_len);
    let verts = make_slice(verts_ptr, verts_byte_len);
    geom.init(buffer, templates, parts, verts);
}

#[export_name = "structure_geom_reset"]
pub extern fn structure_geom_reset(geom: &mut structures::GeomGen,
                                   cx0: i32,
                                   cy0: i32,
                                   cx1: i32,
                                   cy1: i32,
                                   sheet: u8) {
    geom.reset(Region::new(V2::new(cx0, cy0),
                           V2::new(cx1, cy1)),
               sheet);
}

#[export_name = "structure_geom_generate"]
pub unsafe extern fn structure_geom_generate(geom: &mut structures::GeomGen,
                                             buf_ptr: *mut structures::Vertex,
                                             buf_byte_len: usize,
                                             result: &mut GeometryResult) {
    let buf = make_slice_mut(buf_ptr, buf_byte_len);

    let mut idx = 0;
    let more = geom.generate(buf, &mut idx);

    result.vertex_count = idx;
    result.more = more as u8;
}


#[export_name = "light_geom_init"]
pub unsafe extern fn light_geom_init(geom: &mut lights::GeomGen<'static>,
                                     buffer: &'static structures::Buffer<'static>,
                                     templates_ptr: *const gfx_types::StructureTemplate,
                                     templates_byte_len: usize) {
    let templates = make_slice(templates_ptr, templates_byte_len);
    geom.init(buffer, templates);
}

#[export_name = "light_geom_reset"]
pub extern fn light_geom_reset(geom: &mut lights::GeomGen,
                               cx0: i32,
                               cy0: i32,
                               cx1: i32,
                               cy1: i32) {
    geom.reset(Region::new(V2::new(cx0, cy0),
                           V2::new(cx1, cy1)));
}

#[export_name = "light_geom_generate"]
pub unsafe extern fn light_geom_generate(geom: &mut lights::GeomGen,
                                         buf_ptr: *mut lights::Vertex,
                                         buf_byte_len: usize,
                                         result: &mut GeometryResult) {
    let buf = make_slice_mut(buf_ptr, buf_byte_len);

    let mut idx = 0;
    let more = geom.generate(buf, &mut idx);

    result.vertex_count = idx;
    result.more = more as u8;
}


// SIZEOF

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Sizes {
    shape_chunk: usize,
    shape_layers: usize,

    block_data: usize,
    block_chunk: usize,
    local_chunks: usize,

    structure: usize,

    terrain_vertex: usize,
    terrain_geom_gen: usize,

    structures_template: usize,
    template_part: usize,
    template_vertex: usize,
    structures_buffer: usize,
    structures_vertex: usize,
    structures_geom_gen: usize,

    light_vertex: usize,
    light_geom_gen: usize,
}

#[export_name = "get_sizes"]
pub extern fn get_sizes(sizes: &mut Sizes, num_sizes: &mut usize) {
    use core::mem::size_of;

    sizes.shape_chunk = size_of::<ShapeChunk>();
    sizes.shape_layers = size_of::<ShapeLayers>();

    sizes.block_data = size_of::<gfx_types::BlockData>();
    sizes.block_chunk = size_of::<gfx_types::BlockChunk>();
    sizes.local_chunks = size_of::<gfx_types::LocalChunks>();

    sizes.structure = size_of::<structures::Structure>();

    sizes.terrain_vertex = size_of::<terrain::Vertex>();
    sizes.terrain_geom_gen = size_of::<terrain::GeomGen>();

    sizes.structures_template = size_of::<gfx_types::StructureTemplate>();
    sizes.template_part = size_of::<gfx_types::TemplatePart>();
    sizes.template_vertex = size_of::<gfx_types::TemplateVertex>();
    sizes.structures_buffer = size_of::<structures::Buffer>();
    sizes.structures_vertex = size_of::<structures::Vertex>();
    sizes.structures_geom_gen = size_of::<structures::GeomGen>();

    sizes.light_vertex = size_of::<lights::Vertex>();
    sizes.light_geom_gen = size_of::<lights::GeomGen>();

    *num_sizes = size_of::<Sizes>() / size_of::<usize>();
}


#[export_name = "test"]
pub extern fn test_wrapper() {
}
