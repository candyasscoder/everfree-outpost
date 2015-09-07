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
use physics::{CHUNK_SIZE, CHUNK_BITS, CHUNK_MASK, TILE_SIZE, TILE_BITS};
use graphics::{BlockDisplay, BlockData, BlockChunk, LocalChunks};
use graphics::{TerrainVertex, TerrainGeometryBuffer};
use graphics::{StructureTemplate, StructureTemplateData, StructureBuffer,
               StructureVertex, StructureGeometryBuffer};
use graphics::{LightGeometryState, LightVertex, LightGeometryBuffer};

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
                                      template_id: u32) -> usize {
    if let Some(idx) = buf.insert(external_id,
                                  (pos_x, pos_y, pos_z),
                                  template_id) {
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

#[export_name = "structure_buffer_set_oneshot_start"]
pub extern fn structure_buffer_set_oneshot_start(buf: &mut structures::Buffer,
                                                 idx: usize,
                                                 oneshot_start: u16) {
    buf[idx].oneshot_start = oneshot_start;
}


#[export_name = "structure_base_geom_init"]
pub unsafe extern fn structure_base_geom_init(geom: &mut structures::base::GeomGen<'static>,
                                              buffer: &'static structures::Buffer<'static>,
                                              templates_ptr: *const gfx_types::StructureTemplate,
                                              templates_byte_len: usize) {
    let templates = make_slice(templates_ptr, templates_byte_len);
    geom.init(buffer, templates);
}

#[export_name = "structure_base_geom_reset"]
pub extern fn structure_base_geom_reset(geom: &mut structures::base::GeomGen,
                                        cx0: i32,
                                        cy0: i32,
                                        cx1: i32,
                                        cy1: i32,
                                        sheet: u8) {
    geom.reset(Region::new(V2::new(cx0, cy0),
                           V2::new(cx1, cy1)),
               sheet);
}

#[export_name = "structure_base_geom_generate"]
pub unsafe extern fn structure_base_geom_generate(geom: &mut structures::base::GeomGen,
                                                  buf_ptr: *mut structures::base::Vertex,
                                                  buf_byte_len: usize,
                                                  result: &mut GeometryResult) {
    let buf = make_slice_mut(buf_ptr, buf_byte_len);

    let mut idx = 0;
    let more = geom.generate(buf, &mut idx);

    result.vertex_count = idx;
    result.more = more as u8;
}


#[export_name = "structure_anim_geom_init"]
pub unsafe extern fn structure_anim_geom_init(geom: &mut structures::anim::GeomGen<'static>,
                                              buffer: &'static structures::Buffer<'static>,
                                              templates_ptr: *const gfx_types::StructureTemplate,
                                              templates_byte_len: usize) {
    let templates = make_slice(templates_ptr, templates_byte_len);
    geom.init(buffer, templates);
}

#[export_name = "structure_anim_geom_reset"]
pub extern fn structure_anim_geom_reset(geom: &mut structures::anim::GeomGen,
                                        cx0: i32,
                                        cy0: i32,
                                        cx1: i32,
                                        cy1: i32,
                                        sheet: u8) {
    geom.reset(Region::new(V2::new(cx0, cy0),
                           V2::new(cx1, cy1)),
               sheet);
}

#[export_name = "structure_anim_geom_generate"]
pub unsafe extern fn structure_anim_geom_generate(geom: &mut structures::anim::GeomGen,
                                                  buf_ptr: *mut structures::anim::Vertex,
                                                  buf_byte_len: usize,
                                                  result: &mut GeometryResult) {
    let buf = make_slice_mut(buf_ptr, buf_byte_len);

    let mut idx = 0;
    let more = geom.generate(buf, &mut idx);

    result.vertex_count = idx;
    result.more = more as u8;
}





#[export_name = "load_chunk"]
pub extern fn load_chunk(local: &mut LocalChunks,
                         chunk: &BlockChunk,
                         cx: u16,
                         cy: u16) {
    graphics::load_chunk(local, chunk, cx, cy);
}

#[export_name = "generate_terrain_geometry"]
pub extern fn generate_terrain_geometry(local: &LocalChunks,
                                        block_data: &BlockData,
                                        geom: &mut TerrainGeometryBuffer,
                                        cx: u16,
                                        cy: u16) -> usize {
    graphics::generate_geometry(local, block_data, geom, cx, cy, |_, _, _| true)
}

#[export_name = "generate_sliced_terrain_geometry"]
pub extern fn generate_sliced_terrain_geometry(local: &LocalChunks,
                                block_data: &BlockData,
                                geom: &mut TerrainGeometryBuffer,
                                cx: u16,
                                cy: u16,
                                max_z: i32) -> usize {
    graphics::generate_geometry(local, block_data, geom, cx, cy,
                                |pos, _, _| pos.z < max_z)
}

#[export_name = "init_structure_buffer"]
pub extern fn init_structure_buffer(structures: &mut StructureBuffer<'static>,
                                    templates: &'static StructureTemplateData) {
    unsafe { structures.init(templates) };
}

#[export_name = "add_structure"]
pub extern fn add_structure(structures: &mut StructureBuffer,
                            px_x: i32,
                            px_y: i32,
                            px_z: i32,
                            template_id: u32) -> usize {
    let x = px_x / TILE_SIZE % (LOCAL_SIZE * CHUNK_SIZE);
    let y = px_y / TILE_SIZE % (LOCAL_SIZE * CHUNK_SIZE);
    let z = px_z / TILE_SIZE;
    structures.add_structure((x as u8, y as u8, z as u8), template_id)
}

#[export_name = "remove_structure"]
pub extern fn remove_structure(structures: &mut StructureBuffer,
                               idx: usize) {
    structures.remove_structure(idx);
}

#[export_name = "set_structure_oneshot_start"]
pub extern fn set_structure_oneshot_start(structures: &mut StructureBuffer,
                                          idx: usize,
                                          start: u16) {
    structures.set_oneshot_start(idx, start);
}

#[export_name = "reset_structure_geometry"]
pub extern fn reset_structure_geometry(structures: &mut StructureBuffer) {
    structures.start_geometry_gen();
}


pub struct StructureGeometryResult {
    vertex_count: usize,
    sheet: u8,
    more: u8,
}

#[export_name = "generate_structure_geometry"]
pub extern fn generate_structure_geometry(structures: &mut StructureBuffer,
                                          geom: &mut StructureGeometryBuffer,
                                          cx: u8,
                                          cy: u8,
                                          max_z: u8,
                                          output: &mut StructureGeometryResult) {
    let (vertex_count, sheet, more) =
        if max_z >= 16 {
            structures.continue_geometry_gen(geom, cx, cy, |_, _| true)
        } else {
            structures.continue_geometry_gen(geom, cx, cy, |s, _| s.pos.2 < max_z)
        };
    output.vertex_count = vertex_count;
    output.sheet = sheet;
    output.more = more as u8;
}

#[export_name = "generate_structure_anim_geometry"]
pub extern fn generate_structure_anim_geometry(structures: &mut StructureBuffer,
                                               geom: &mut StructureGeometryBuffer,
                                               cx: u8,
                                               cy: u8,
                                               max_z: u8,
                                               output: &mut StructureGeometryResult) {
    let (vertex_count, sheet, more) =
        if max_z >= 16 {
            structures.continue_anim_geometry_gen(geom, cx, cy, |_, _| true)
        } else {
            structures.continue_anim_geometry_gen(geom, cx, cy, |s, _| s.pos.2 < max_z)
        };
    output.vertex_count = vertex_count;
    output.sheet = sheet;
    output.more = more as u8;
}


#[export_name = "init_light_state"]
pub extern fn init_light_state(light_state: &mut LightGeometryState<'static>,
                               block_data: &'static BlockData,
                               templates: &'static StructureTemplateData) {
    unsafe { light_state.init(block_data, templates) };
}

#[export_name = "reset_light_geometry"]
pub extern fn reset_light_geometry(light_state: &mut LightGeometryState,
                                   cx0: u8,
                                   cy0: u8,
                                   cx1: u8,
                                   cy1: u8) {
    light_state.start_geometry_gen(
        Region::new(V2::new(cx0 as i32, cy0 as i32),
                    V2::new(cx1 as i32, cy1 as i32)));
}

pub struct LightGeometryResult {
    vertex_count: usize,
    more: u8,
}

#[export_name = "generate_light_geometry"]
pub extern fn generate_light_geometry(light_state: &mut LightGeometryState,
                                      geom: &mut LightGeometryBuffer,
                                      local: &LocalChunks,
                                      structure_buffer: &StructureBuffer,
                                      output: &mut LightGeometryResult) {
    let (count, more) = light_state.generate_geometry(geom,
                                                      local,
                                                      structure_buffer.structures());
    output.vertex_count = count;
    output.more = more as u8;
}


// SIZEOF

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Sizes {
    shape_chunk: usize,
    shape_layers: usize,

    block_display: usize,
    block_data: usize,
    block_chunk: usize,
    local_chunks: usize,

    terrain_vertex: usize,
    terrain_geometry_buffer: usize,

    structure_template: usize,
    structure_template_data: usize,
    structure_buffer: usize,
    structure_vertex: usize,
    structure_geometry_buffer: usize,

    light_geometry_state: usize,
    light_vertex: usize,
    light_geometry_buffer: usize,

    terrain2_vertex: usize,
    terrain2_geom_gen: usize,

    structures2_template: usize,
    structures2_buffer: usize,
    structures2_base_vertex: usize,
    structures2_base_geom_gen: usize,
    structures2_anim_vertex: usize,
    structures2_anim_geom_gen: usize,
}

#[export_name = "get_sizes"]
pub extern fn get_sizes(sizes: &mut Sizes, num_sizes: &mut usize) {
    use core::mem::size_of;

    sizes.shape_chunk = size_of::<ShapeChunk>();
    sizes.shape_layers = size_of::<ShapeLayers>();

    sizes.block_display = size_of::<BlockDisplay>();
    sizes.block_data = size_of::<BlockData>();
    sizes.block_chunk = size_of::<BlockChunk>();
    sizes.local_chunks = size_of::<LocalChunks>();

    sizes.terrain_vertex = size_of::<TerrainVertex>();
    sizes.terrain_geometry_buffer = size_of::<TerrainGeometryBuffer>();

    sizes.structure_template = size_of::<StructureTemplate>();
    sizes.structure_template_data = size_of::<StructureTemplateData>();
    sizes.structure_buffer = size_of::<StructureBuffer>();
    sizes.structure_vertex = size_of::<StructureVertex>();
    sizes.structure_geometry_buffer = size_of::<StructureGeometryBuffer>();

    sizes.light_geometry_state = size_of::<LightGeometryState>();
    sizes.light_vertex = size_of::<LightVertex>();
    sizes.light_geometry_buffer = size_of::<LightGeometryBuffer>();

    sizes.terrain2_vertex = size_of::<terrain::Vertex>();
    sizes.terrain2_geom_gen = size_of::<terrain::GeomGen>();

    sizes.structures2_template = size_of::<gfx_types::StructureTemplate>();
    sizes.structures2_buffer = size_of::<structures::Buffer>();
    sizes.structures2_base_vertex = size_of::<structures::base::Vertex>();
    sizes.structures2_base_geom_gen = size_of::<structures::base::GeomGen>();
    sizes.structures2_anim_vertex = size_of::<structures::anim::Vertex>();
    sizes.structures2_anim_geom_gen = size_of::<structures::anim::GeomGen>();

    *num_sizes = size_of::<Sizes>() / size_of::<usize>();
}


#[export_name = "test"]
pub extern fn test_wrapper() {
}
