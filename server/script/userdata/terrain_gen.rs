use std::ptr;
use rand::{self, XorShiftRng};

use physics::CHUNK_SIZE;

use types::*;
use util::StrResult;

use lua::LuaState;
use script::Nil;
use script::traits::Userdata;
use script::userdata::{Wrapper, OptWrapper};
use terrain_gen;


pub type Rng = Wrapper<XorShiftRng>;

impl_type_name!(Rng);
impl_metatable_key!(Rng);

impl Userdata for Rng {
    fn populate_table(lua: &mut LuaState) {
        lua_table_fns2! {
            lua, -1,

            fn new() -> Rng {
                Rng::new(rand::random())
            }

            fn with_seed(!partial ctx: &terrain_gen::TerrainGen, seed: i32) -> Rng {
                Rng::new(ctx.rng(seed as u32))
            }

            // TODO: fix mk_block macro so we don't need double braces here.
            fn gen(rng: &Rng,
                   min: i32,
                   max: i32) -> i32 {
                rng.open(|r| {
                    use rand::Rng;
                    r.gen_range(min, max)
                })
            }
        }

        insert_function!(lua, -1, "choose", rng_choose);
        insert_function!(lua, -1, "choose_weighted", rng_choose_weighted);
    }

    fn populate_metatable(lua: &mut LuaState) {
        lua_table_fns2! {
            lua, -1,

            fn __gc(x: &Rng) -> () {
                unsafe { ptr::read(x as *const _) };
            }
        }
    }
}

/// Perform reservoir sampling over a Lua iterator.  Returns all values produced for the chosen
/// iteration.  (For example, `rng:choose(pairs(table))` returns the chosen key-value pair.)
fn rng_choose(mut lua: LuaState) -> ::libc::c_int {
    use std::iter::count;
    use std::mem;
    use libc::c_int;
    use lua::{self, ValueType};
    use script::traits::FromLua;

    if lua.top_index() != 4 {
        lua.push_string("wrong number of arguments for 'choose' (4 expected)");
        unsafe { lua.error() };
    }

    const RNG_IDX: c_int = 1;
    const F_IDX: c_int = 2;
    const S_IDX: c_int = 3;
    const VAR_IDX: c_int = 4;
    const VARS_BASE_IDX: c_int = 5;

    let mut ok = true;
    let mut last_size = 0;
    {
        let rng = unsafe {
            <&Rng as FromLua>::check(&mut lua, 1, "choose");
            let rng = FromLua::from_lua(&mut lua, 1);
            // Discard lifetime.
            mem::transmute::<&Rng, &Rng>(rng)
        };
        let mut rng = rng.x.borrow_mut();
        // From now on, we need to be careful not to accidentally pop the Rng userdata.

        // This loop emulates the behavior of the Lua `for` statement.
        for i in count(1, 1) {
            lua.copy(F_IDX);
            lua.copy(S_IDX);
            lua.copy(VAR_IDX);
            match lua.pcall(2, lua::MULTRET, 0) {
                Ok(()) => {},
                Err(_) => {
                    ok = false;
                    break;
                },
            }

            // Stack layout:
            //   1: Rng userdata
            //   2: f
            //   3: s
            //   4: val
            //
            //   5: old_val1
            //   ...
            //   5+n: old_valn
            //
            //   5+n+1: new_val1
            //   ...
            //   5+n+m: new_valm

            let new_base = VARS_BASE_IDX + last_size;
            let new_size = lua.top_index() - new_base + 1;
            if new_size == 0 || lua.type_of(new_base) == ValueType::Nil {
                lua.pop(new_size);
                break;
            }

            lua.copy(new_base);
            lua.replace(VAR_IDX);

            let keep = {
                use rand::Rng;
                rng.gen_range(0, i) == 0
            };
            if keep {
                // Copy the new values over the top of the old values, then pop the extra `n`.
                //   State: ... A1 A2 B1 B2 B3
                for j in range(0, new_size) {
                    let old_idx = VARS_BASE_IDX + j;
                    let new_idx = new_base + j;
                    lua.copy(new_idx);
                    lua.replace(old_idx);
                }
                //   State: ... B1 B2 B3 B2 B3
                lua.pop(last_size);
                last_size = new_size;
                //   State: ... B1 B2 B3
            } else {
                // Keep old value.
                lua.pop(new_size);
            }
        }
    }

    if !ok {
        // Rng's borrow_mut guard is out of scope already.  Nothing else on the stack needs a
        // destructor.
        //
        // The error message is left on the stack when breaking, so we can just call `error`.
        unsafe { lua.error() };
    }

    for j in range(0, last_size) {
        let old_idx = 1 + j;
        let new_idx = VARS_BASE_IDX + j;
        lua.copy(new_idx);
        lua.replace(old_idx);
    }
    lua.pop(VARS_BASE_IDX - 1);
    return last_size;
}

/// Perform reservoir sampling to choose a weighted value from a Lua iterator.  The iterator should
/// produce (value, weight) pairs.  The chosen value will be returned.
fn rng_choose_weighted(mut lua: LuaState) -> ::libc::c_int {
    use std::mem;
    use libc::c_int;
    use lua::ValueType;
    use script::traits::FromLua;

    if lua.top_index() != 4 {
        lua.push_string("wrong number of arguments for 'choose_weighted' (4 expected)");
        unsafe { lua.error() };
    }

    const RNG_IDX: c_int = 1;
    const F_IDX: c_int = 2;
    const S_IDX: c_int = 3;
    const VAR_IDX: c_int = 4;
    const CHOSEN_VAL_IDX: c_int = 5;
    const NEW_VAL_IDX: c_int = 6;
    const NEW_WEIGHT_IDX: c_int = 7;

    lua.push_nil();

    let mut ok = true;
    {
        let rng = unsafe {
            <&Rng as FromLua>::check(&mut lua, 1, "choose");
            let rng = FromLua::from_lua(&mut lua, 1);
            mem::transmute::<&Rng, &Rng>(rng)
        };
        // Discard lifetime.
        let mut rng = rng.x.borrow_mut();
        // From now on, we need to be careful not to accidentally pop the Rng userdata.

        let mut total_weight = 0;

        // This loop emulates the behavior of the Lua `for` statement.
        loop {
            lua.copy(F_IDX);
            lua.copy(S_IDX);
            lua.copy(VAR_IDX);
            match lua.pcall(2, 2, 0) {
                Ok(()) => {},
                Err(_) => {
                    ok = false;
                    break;
                },
            }

            if lua.type_of(NEW_VAL_IDX) == ValueType::Nil {
                lua.pop(2);
                break;
            }

            lua.copy(NEW_VAL_IDX);
            lua.replace(VAR_IDX);

            let weight = lua.to_integer(NEW_WEIGHT_IDX);
            total_weight += weight;

            let keep = {
                use rand::Rng;
                rng.gen_range(0, total_weight) < weight
            };
            if keep {
                lua.copy(NEW_VAL_IDX);
                lua.replace(CHOSEN_VAL_IDX);
            }
            lua.pop(2);
        }
    }

    if !ok {
        // Rng's borrow_mut guard is out of scope already.  Nothing else on the stack needs a
        // destructor.
        //
        // The error message is left on the stack when breaking, so we can just call `error`.
        unsafe { lua.error() };
    }

    lua.copy(CHOSEN_VAL_IDX);
    lua.replace(1);
    lua.pop(4);
    return 1;
}


pub type GenChunk = OptWrapper<terrain_gen::GenChunk>;

impl_type_name!(GenChunk);
impl_metatable_key!(GenChunk);

impl Userdata for GenChunk {
    fn populate_table(lua: &mut LuaState) {
        lua_table_fns2! {
            lua, -1,

            fn set_block(!partial ctx: &terrain_gen::TerrainGen,
                         gc: &GenChunk,
                         pos: V3,
                         block: &str) -> StrResult<()> {
                let block_id = unwrap!(ctx.data().block_data.find_id(block));

                let bounds = Region::new(scalar(0), scalar(CHUNK_SIZE));
                if !bounds.contains(pos) {
                    fail!("position out of bounds");
                };

                let idx = bounds.index(pos);
                try!(gc.open(|gc| gc.blocks[idx] = block_id));
                Ok(())
            }

            fn add_structure(!partial ctx: &terrain_gen::TerrainGen,
                             gc: &GenChunk,
                             pos: V3,
                             template_name: &str) -> StrResult<i32> {
                let template_id = unwrap!(ctx.data().object_templates.find_id(template_name));

                let bounds = Region::new(scalar(0), scalar(CHUNK_SIZE));
                if !bounds.contains(pos) {
                    fail!("position out of bounds");
                };

                // TODO: could use some sanity checks here.
                let s = terrain_gen::GenStructure::new(pos, template_id);
                let idx = try!(gc.open(|gc| {
                    gc.structures.push(s);
                    gc.structures.len() - 1
                }));
                Ok(idx as i32)
            }

            fn set_structure_extra(gc: &GenChunk,
                                   index: i32,
                                   key: &str,
                                   value: &str) -> StrResult<()> {
                try!(try!(gc.open(|gc| -> StrResult<_> {
                    let s = unwrap!(gc.structures.get_mut(index as usize));
                    s.extra.insert(String::from_str(key), String::from_str(value));
                    Ok(())
                })));
                Ok(())
            }

            fn get_structure_extra(gc: &GenChunk,
                                   index: i32,
                                   key: &str) -> StrResult<String> {
                let value = try!(try!(gc.open(|gc| -> StrResult<_> {
                    let s = unwrap!(gc.structures.get(index as usize));
                    let value = unwrap!(s.extra.get(key));
                    Ok(String::from_str(value))
                })));
                Ok(value)
            }
        }
    }

    fn populate_metatable(lua: &mut LuaState) {
        lua_table_fns2! {
            lua, -1,

            fn __gc(x: &GenChunk) -> () {
                // Run destructor on `x`.  After this, the memory will be freed by Lua.
                unsafe { ptr::read(x as *const _) };
            }
        }
    }
}


pub struct Values {
    v: Vec<i32>,
}

impl_type_name!(Values);
impl_metatable_key!(Values);

impl Userdata for Values {
    fn populate_metatable(lua: &mut LuaState) {
        lua_table_fns2! {
            lua, -1,

            fn __len(values: &Values, __: Nil) -> u32 {
                values.v.len() as u32
            }

            fn __index(values: &Values, idx: u32) -> Option<i32> {
                if idx == 0 {
                    // Avoid panic from underflow.
                    return None;
                };
                values.v.get(idx as usize - 1).map(|&x| x)
            }

            fn __gc(x: &Values) -> () {
                // Run destructor on `x`.  After this, the memory will be freed by Lua.
                unsafe { ptr::read(x as *const _) };
            }
        }
    }
}

pub type ValuesMut = OptWrapper<Vec<i32>>;

impl_type_name!(ValuesMut);
impl_metatable_key!(ValuesMut);

impl Userdata for ValuesMut {
    fn populate_table(lua: &mut LuaState) {
        lua_table_fns2! {
            lua, -1,

            fn new() -> ValuesMut {
                ValuesMut::new(Vec::new())
            }

            fn push(vs: &ValuesMut, v: i32) -> StrResult<()> {
                vs.open(|vs| vs.push(v))
            }
        }
    }

    fn populate_metatable(lua: &mut LuaState) {
        lua_table_fns2! {
            lua, -1,

            fn __gc(x: &ValuesMut) -> () {
                // Run destructor on `x`.  After this, the memory will be freed by Lua.
                unsafe { ptr::read(x as *const _) };
            }
        }
    }
}


pub struct Points {
    p: Vec<V2>,
}

impl_type_name!(Points);
impl_metatable_key!(Points);

impl Userdata for Points {
    fn populate_metatable(lua: &mut LuaState) {
        lua_table_fns2! {
            lua, -1,

            fn __len(points: &Points, __: Nil) -> u32 {
                points.p.len() as u32
            }

            fn __index(points: &Points, idx: u32) -> Option<V2> {
                points.p.get(idx as usize - 1).map(|&x| x)
            }

            fn __gc(x: &Points) -> () {
                // Run destructor on `x`.  After this, the memory will be freed by Lua.
                unsafe { ptr::read(x as *const _) };
            }
        }
    }
}


// Field wrappers

fn field_get_value<F: terrain_gen::Field>(field: &F, pos: V2) -> i32 {
    use terrain_gen::Field;
    field.get_value(pos)
}

fn field_get_region<F: terrain_gen::Field>(field: &F, min: V2, max: V2) -> Values {
    use std::iter;
    use terrain_gen::Field;

    let bounds = Region2::new(min, max);
    let mut buf = iter::repeat(0).take(bounds.volume() as usize).collect::<Vec<_>>();

    field.get_region(bounds, &mut *buf);

    Values {
        v: buf,
    }
}


pub type Field = OptWrapper<Box<terrain_gen::Field>>;

impl_type_name!(Field);
impl_metatable_key!(Field);

impl Userdata for Field {
    fn populate_table(lua: &mut LuaState) {
        lua_table_fns2! {
            lua, -1,

            fn get_value(field: &Field, pos: V2) -> StrResult<i32> {
                field.open(|f| field_get_value(f, pos))
            }

            fn get_region(field: &Field, min: V2, max: V2) -> StrResult<Values> {
                field.open(|f| field_get_region(f, min, max))
            }
        }
    }

    fn populate_metatable(lua: &mut LuaState) {
        lua_table_fns2! {
            lua, -1,

            fn __gc(x: &Field) -> () {
                unsafe { ptr::read(x as *const _) };
            }
        }
    }
}

macro_rules! define_field {
    ($Field:ident = $tg_Field:ty: ($($arg:ident: $Arg:ty),*) $($body:tt)*) => {
        pub type $Field = OptWrapper<$tg_Field>;

        impl_type_name!($Field);
        impl_metatable_key!($Field);

        impl Userdata for $Field {
            fn populate_table(lua: &mut LuaState) {
                lua_table_fns2! {
                    lua, -1,

                    fn new($($arg: $Arg),*) -> StrResult<$Field> {
                        let result = {
                            use terrain_gen::$Field;
                            mk_block!($($body)* {})
                        };
                        Ok($Field::new(result))
                    }

                    fn get_value(field: &$Field, pos: V2) -> StrResult<i32> {
                        field.open(|f| field_get_value(f, pos))
                    }

                    fn get_region(field: &$Field, min: V2, max: V2) -> StrResult<Values> {
                        field.open(|f| field_get_region(f, min, max))
                    }

                    fn upcast(field: &$Field) -> StrResult<Field> {
                        let f = unwrap!(field.take());
                        Ok(Field::new(Box::new(f) as Box<terrain_gen::Field>))
                    }
                }
            }

            fn populate_metatable(lua: &mut LuaState) {
                lua_table_fns2! {
                    lua, -1,

                    fn __gc(x: &$Field) -> () {
                        unsafe { ptr::read(x as *const _) };
                    }
                }
            }
        }
    };

    ($Field:ident: ($($arg:ident: $Arg:ty),*) $($body:tt)*) => {
        define_field!($Field = terrain_gen::$Field: ($($arg: $Arg),*) $($body)*);
    };
}

macro_rules! define_wrapper_field {
    ($Field:ident: $($x:tt)*) => {
        define_field!($Field = terrain_gen::$Field<Box<terrain_gen::Field>>: $($x)*);
    };
}

define_field!(ConstantField: (val: i32) { ConstantField(val) });
define_field!(RandomField: (seed0: u32, seed1: u32, min: i32, max: i32) {
    if min > max {
        fail!("expected min <= max");
    };
    let seed = ((seed0 as u64) << 32) | (seed1 as u64);
    // max+1 to be consistent with lua conventions (inclusive rather than exclusive).
    RandomField::new(seed, min, max + 1)
});

define_wrapper_field!(FilterField: (base: &Field, min: i32, max: i32) {
    let base = unwrap!(base.take());
    // max+1 to be consistent with lua conventions (inclusive rather than exclusive).
    FilterField::new(base, min, max + 1)
});
define_wrapper_field!(BorderField: (base: &Field) {
    let base = unwrap!(base.take());
    BorderField::new(base)
});

define_wrapper_field!(DiamondSquare: (seed0: u32, seed1: u32, init: &Field, offsets: &ValuesMut) {
    let init = unwrap!(init.take());
    let offsets = unwrap!(offsets.take());
    let seed = ((seed0 as u64) << 32) | (seed1 as u64);
    DiamondSquare::new(seed, init, offsets)
});



pub struct IsoDiskSampler {
    s: terrain_gen::IsoDiskSampler<Box<terrain_gen::Field>>,
}

impl_type_name!(IsoDiskSampler);
impl_metatable_key!(IsoDiskSampler);

impl Userdata for IsoDiskSampler {
    fn populate_table(lua: &mut LuaState) {
        lua_table_fns2! {
            lua, -1,

            fn new_constant(seed: i32,
                            spacing: u16,
                            chunk_size: u16) -> IsoDiskSampler {
                let spacing_field =
                    Box::new(terrain_gen::ConstantField(spacing as i32))
                    as Box<terrain_gen::Field>;
                let sampler = terrain_gen::IsoDiskSampler::new(seed as u64,
                                                               spacing,
                                                               spacing,
                                                               chunk_size,
                                                               spacing_field);
                IsoDiskSampler { s: sampler }
            }

            fn get_points(sampler: &IsoDiskSampler,
                          min: V2,
                          max: V2) -> Points {{
                use terrain_gen::PointSource;
                Points { p: sampler.s.generate_points(Region2::new(min, max)) }
            }}
        }
    }

    fn populate_metatable(lua: &mut LuaState) {
        lua_table_fns2! {
            lua, -1,

            fn __gc(x: &IsoDiskSampler) -> () {
                unsafe { ptr::read(x as *const _) };
            }
        }
    }
}
