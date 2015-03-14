use std::cell::RefCell;
use std::ptr;
use rand::{self, XorShiftRng, SeedableRng};

use physics::{TILE_SIZE, CHUNK_SIZE};

use types::*;

use lua::LuaState;
use script::traits::Userdata;
use terrain_gen;


pub struct Rng {
    rng: RefCell<XorShiftRng>,
}

impl Rng {
    pub fn new(rng: XorShiftRng) -> Rng {
        Rng {
            rng: RefCell::new(rng),
        }
    }
}

impl_type_name!(Rng);
impl_metatable_key!(Rng);

impl Userdata for Rng {
    fn populate_table(lua: &mut LuaState) {
        lua_table_fns2! {
            lua, -1,

            fn new() -> Rng {
                Rng {
                    rng: RefCell::new(rand::random()),
                }
            }

            fn with_seed(seed: i32) -> Rng {
                let rng = SeedableRng::from_seed([seed as u32,
                                                  0xde4abf57,
                                                  0xe83c9492,
                                                  0xb3b71499]);
                Rng {
                    rng: RefCell::new(rng),
                }
            }

            // TODO: fix mk_block macro so we don't need double braces here.
            fn gen(rng: &Rng,
                   min: i32,
                   max: i32) -> i32 {{
                use rand::Rng;
                rng.rng.borrow_mut().gen_range(min, max)
            }}
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
            mem::transmute::<&Rng, &Rng>(rng)
        };
        // Discard lifetime.
        let mut rng = rng.rng.borrow_mut();
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
    use std::iter::count;
    use std::mem;
    use libc::c_int;
    use lua::{self, ValueType};
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
        let mut rng = rng.rng.borrow_mut();
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


pub struct GenChunk {
    blocks: RefCell<Option<Box<BlockChunk>>>,
}

impl GenChunk {
    pub fn new() -> GenChunk {
        GenChunk {
            blocks: RefCell::new(Some(Box::new(EMPTY_CHUNK))),
        }
    }

    pub fn take(&self) -> Option<Box<BlockChunk>> {
        self.blocks.borrow_mut().take()
    }
}

impl_type_name!(GenChunk);
impl_metatable_key!(GenChunk);

impl Userdata for GenChunk {
    fn populate_table(lua: &mut LuaState) {
        lua_table_fns2! {
            lua, -1,

            fn new() -> GenChunk { GenChunk::new() }

            fn set_block(!partial ctx: &terrain_gen::Context,
                         gc: &GenChunk,
                         pos: V3,
                         block: &str) -> bool {
                let block_id = unwrap_or!(ctx.data().block_data.find_id(block),
                                          return false);

                let bounds = Region::new(scalar(0), scalar(CHUNK_SIZE));
                if !bounds.contains(pos) {
                    return false;
                };

                let idx = bounds.index(pos);
                let mut b = gc.blocks.borrow_mut();
                let b = unwrap_or!(b.as_mut(), return false);
                b[idx] = block_id;
                true
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
