#![crate_name = "backend"]
#![feature(phase)]
#![feature(tuple_indexing, if_let)]
#![feature(macro_rules)]
#![allow(non_upper_case_globals)]

#[phase(plugin, link)]
extern crate log;
extern crate time;

extern crate physics;

use std::collections::HashMap;
use std::collections::hashmap::Occupied;
use std::io;
use std::io::{BufReader, BufWriter};
use std::io::IoResult;
use std::mem;
use std::rand::{StdRng, Rng};
use std::u16;

use physics::{CHUNK_SIZE, CHUNK_BITS, TILE_SIZE};
use physics::v3::{V3, scalar};

use wire::{WireReader, WireWriter};
use msg::Motion as WireMotion;

mod msg;
mod wire;

pub type Time = u16;

fn main() {
    real_main().unwrap()
}


const CHUNK_TOTAL: uint = 1 << (3 * CHUNK_BITS);
type Chunk = [u16, ..CHUNK_TOTAL];

const LOCAL_BITS: uint = 3;
const LOCAL_SIZE: i32 = 1 << LOCAL_BITS;
const LOCAL_MASK: i32 = LOCAL_SIZE - 1;
const LOCAL_TOTAL: uint = 1 << (2 * LOCAL_BITS);

struct State {
    map: [Chunk, ..LOCAL_TOTAL],
    clients: HashMap<u16, Client>,
}


fn real_main() -> IoResult<()> {
    let mut input = WireReader::new(io::stdin());
    let mut output = WireWriter::new(io::BufferedWriter::new(io::stdout().unwrap()));

    let mut rng = try!(StdRng::new());

    let mut state = State {
        map: [[0, ..CHUNK_TOTAL], ..LOCAL_TOTAL],
        clients: HashMap::new(),
    };

    loop {
        let (id, req) = try!(msg::Request::read_from(&mut input));
        match req {
            msg::GetTerrain => {
                for c in range(0, 8 * 8) {
                    let mut data = Vec::from_elem(16 * 16 + 2, 0u16);
                    let len = data.len();
                    for i in range(0, 16 * 16) {
                        if rng.gen_range(0, 10) == 0u8 {
                            data[i] = 0;
                        } else {
                            data[i] = 1;
                        }
                    }
                    data[len - 2] = 0xf000 | (16 * 16 * 15);
                    data[len - 1] = 0;

                    let resp = msg::TerrainChunk(c, data);
                    try!(resp.write_to(id, &mut output));
                }
            },

            msg::UpdateMotion(wire_motion) => {
                if let Occupied(mut entry) = state.clients.entry(id) {
                    let motion = entry.get().decode_wire_motion(wire_motion.start_time,
                                                                &wire_motion);
                    log!(10, "client {} reports motion: {} @ {} -> {} @ +{}",
                         id,
                         motion.start_pos, motion.start_time,
                         motion.end_pos, motion.end_time - motion.start_time);
                    entry.get_mut().motion = motion;

                    let mut motion = motion;
                    motion.start_pos = motion.start_pos + V3::new(100, 0, 0);
                    motion.end_pos = motion.end_pos + V3::new(100, 0, 0);
                    let wire_motion2 = entry.get().encode_wire_motion(wire_motion.start_time,
                                                                      &motion);

                    log!(10, "  recv: {}", wire_motion);
                    log!(10, "  send: {}", wire_motion2);
                    try!(msg::PlayerMotion(0, wire_motion2).write_to(id, &mut output));
                } else {
                    warn!("got UpdateMotion for nonexistent client {}", id);
                }
            },

            msg::Ping(cookie) => {
                try!(msg::Pong(cookie, now()).write_to(id, &mut output));
            },

            msg::Input(time, input) => {
                log!(10, "client {} sends input {:x} at time {}",
                     id, input, time);
            },

            msg::AddClient => {
                let client = Client::new(id, scalar(0));

                let inserted = state.clients.insert(id, client);
                if !inserted {
                    warn!("tried to add client {}, but that client is already connected", id);
                }
            },

            msg::RemoveClient => {
                let removed = state.clients.remove(&id);
                if !removed {
                    warn!("tried to remove client {}, but that client is not connected", id);
                }
                msg::ClientRemoved.write_to(id, &mut output);
            },

            msg::BadMessage(opcode) => {
                warn!("unrecognized opcode from client {}: {:x}",
                      id, opcode.unwrap());
            },
        }
    }
}

fn now() -> u16 {
    let timespec = time::get_time();
    (timespec.sec as u16 * 1000) + (timespec.nsec / 1000000) as u16
}

fn convert(s: &[u16]) -> &[u8] {
    use std::mem;
    use std::raw::Slice;
    unsafe {
        mem::transmute(Slice {
            data: s.as_ptr() as *const u8,
            len: s.len() * 2,
        })
    }
}


struct Motion {
    start_pos: V3,
    start_time: Time,
    end_pos: V3,
    end_time: Time,
}

impl Motion {
    fn pos(&self, time: Time) -> V3 {
        let total = self.end_time - self.start_time;
        let current = time - self.start_time;

        if current < total {
            let offset = (self.end_pos - self.start_pos) *
                    scalar(current as i32) / scalar(total as i32);
            self.start_pos + offset
        } else {
            self.end_pos
        }
    }
}

struct Client {
    id: u16,
    motion: Motion,
    base_offset: (u8, u8),
}

impl Client {
    fn new(id: u16, pos: V3) -> Client {
        Client {
            id: id,
            motion: Motion {
                start_pos: pos,
                start_time: 0,
                end_pos: pos,
                end_time: u16::MAX,
            },
            base_offset: (0, 0),
        }
    }

    fn world_base_chunk(&self, now: Time) -> V3 {
        let pos = self.motion.pos(now);

        let size = CHUNK_SIZE * TILE_SIZE;
        let chunk_x = (pos.x + if pos.x < 0 { -size + 1 } else { 0 }) / size;
        let chunk_y = (pos.y + if pos.y < 0 { -size + 1 } else { 0 }) / size;

        let offset = LOCAL_SIZE / 2;

        V3::new(chunk_x - offset, chunk_y - offset, 0)
    }

    fn local_base_chunk(&self, now: Time) -> V3 {
        let base_off = V3::new(self.base_offset.0 as i32,
                               self.base_offset.1 as i32,
                               0);
        (self.world_base_chunk(now) + base_off) & scalar(LOCAL_MASK)
    }

    fn local_to_world(&self, now: Time, local: V3) -> V3 {
        local_to_world(local,
                       self.world_base_chunk(now),
                       self.local_base_chunk(now))
    }

    fn world_to_local(&self, now: Time, world: V3) -> V3 {
        world_to_local(world,
                       self.world_base_chunk(now),
                       self.local_base_chunk(now))
    }

    fn decode_wire_motion(&self, now: Time, wire: &WireMotion) -> Motion {
        let local_start = V3::new(wire.start_pos.0 as i32,
                                  wire.start_pos.1 as i32,
                                  wire.start_pos.2 as i32);
        let local_end = V3::new(wire.end_pos.0 as i32,
                                wire.end_pos.1 as i32,
                                wire.end_pos.2 as i32);

        let world_start = self.local_to_world(now, local_start);
        let world_end = self.local_to_world(now, local_end);

        Motion {
            start_pos: world_start,
            start_time: wire.start_time,
            end_pos: world_end,
            end_time: wire.end_time,
        }
    }

    fn encode_wire_motion(&self, now: Time, motion: &Motion) -> WireMotion {
        let local_start = self.world_to_local(now, motion.start_pos);
        let local_end = self.world_to_local(now, motion.end_pos);


        WireMotion {
            start_pos: (local_start.x as u16,
                        local_start.y as u16,
                        local_start.z as u16),
            start_time: motion.start_time,
            end_pos: (local_end.x as u16,
                      local_end.y as u16,
                      local_end.z as u16),
            end_time: motion.end_time,
        }
    }
}

fn local_to_world(local: V3, world_base_chunk: V3, local_base_chunk: V3) -> V3 {
    let world_base = world_base_chunk * scalar(CHUNK_SIZE * TILE_SIZE);
    let local_base = local_base_chunk * scalar(CHUNK_SIZE * TILE_SIZE);

    let offset = (local - local_base) & scalar(CHUNK_SIZE * TILE_SIZE * LOCAL_SIZE - 1);
    world_base + offset
}

fn world_to_local(world: V3, world_base_chunk: V3, local_base_chunk: V3) -> V3 {
    let world_base = world_base_chunk * scalar(CHUNK_SIZE * TILE_SIZE);
    let local_base = local_base_chunk * scalar(CHUNK_SIZE * TILE_SIZE);

    let offset = world - world_base;
    local_base + offset
}
