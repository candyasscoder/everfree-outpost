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

fn main() {
    real_main().unwrap()
}


pub type Time = u16;


#[deriving(PartialEq, Eq, Show)]
struct Opcode(u16);

impl Opcode {
    pub fn unwrap(self) -> u16 {
        let Opcode(v) = self;
        v
    }
}

macro_rules! opcodes {
    ($($name:ident = $value:expr,)*) => {
        $(
            #[allow(non_upper_case_globals, dead_code)]
            const $name: Opcode = Opcode($value);
        )*
    }
}

opcodes! {
    GetTerrain = 0x0001,
    UpdateMotion = 0x0002,
    Ping = 0x0003,
    Input = 0x0004,

    TerrainChunk = 0x8001,
    PlayerMotion = 0x8002,
    Pong = 0x8003,

    AddClient = 0xff00,
    RemoveClient = 0xff01,
    ClientRemoved = 0xff02,
}


fn read_msg<R: Reader>(r: &mut R) -> IoResult<(u16, Opcode, Vec<u8>)> {
    let id = try!(r.read_le_u16());
    let size = try!(r.read_le_u16());
    let opcode = try!(r.read_le_u16());
    let body = try!(r.read_exact(size as uint - 2));
    Ok((id, Opcode(opcode), body))
}

fn write_msg<W: Writer>(w: &mut W, id: u16, opcode: Opcode, body: &[u8]) -> IoResult<()> {
    assert!(body.len() + 2 < std::u16::MAX as uint);
    let size = body.len() as u16 + 2;
    try!(w.write_le_u16(id));
    try!(w.write_le_u16(size));
    try!(w.write_le_u16(opcode.unwrap()));
    try!(w.write(body));
    try!(w.flush());
    Ok(())
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
    let mut stdin = io::stdin();
    let mut stdout = io::BufferedWriter::new(io::stdout().unwrap());

    let mut rng = try!(StdRng::new());

    let mut state = State {
        map: [[0, ..CHUNK_TOTAL], ..LOCAL_TOTAL],
        clients: HashMap::new(),
    };

    loop {
        let (id, opcode, body) = try!(read_msg(&mut stdin));

        match opcode {
            GetTerrain => {
                for c in range(0, 8 * 8) {
                    let mut data = Vec::from_elem(1 + 16 * 16 + 2, 0u16);
                    let len = data.len();
                    data[0] = c;
                    for i in range(0, 16 * 16) {
                        if rng.gen_range(0, 10) == 0u8 {
                            data[1 + i] = 0;
                        } else {
                            data[1 + i] = 1;
                        }
                    }
                    data[len - 2] = 0xf000 | (16 * 16 * 15);
                    data[len - 1] = 0;
                    try!(write_msg(&mut stdout, id, TerrainChunk, convert(data.as_slice())));
                }
            },

            UpdateMotion => {
                if let Occupied(mut entry) = state.clients.entry(id) {
                    let wire_motion: WireMotion = try!(Struct::decode_from(body.as_slice()));
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
                    let mut msg = Vec::from_elem(0u16.size() + wire_motion2.size(), 0);
                    try!((0u16, wire_motion2).encode_to(msg.as_mut_slice()));
                    try!(write_msg(&mut stdout, id, PlayerMotion, msg.as_slice()));
                } else {
                    warn!("got UpdateMotion for nonexistent client {}", id);
                }
            },

            Ping => {
                let cookie: u16 = try!(Struct::decode_from(body.as_slice()));
                let mut msg = Vec::from_elem(4, 0);
                try!((cookie, now()).encode_to(msg.as_mut_slice()));
                try!(write_msg(&mut stdout, id, Pong, msg.as_slice()));
            },

            Input => {
                let (time, input): (u16, u16) = try!(Struct::decode_from(body.as_slice()));
                log!(10, "client {} sends input {:x} at time {}",
                     id, input, time);
            },

            AddClient => {
                let client = Client::new(id, scalar(0));

                let inserted = state.clients.insert(id, client);
                if !inserted {
                    warn!("tried to add client {}, but that client is already connected", id);
                }
            },

            RemoveClient => {
                let removed = state.clients.remove(&id);
                if !removed {
                    warn!("tried to remove client {}, but that client is not connected", id);
                }
                try!(write_msg(&mut stdout, id, ClientRemoved, &[]));
            },

            _ => {
                warn!("unrecognized opcode from client {}: {:x} ({} bytes)",
                      id, opcode.unwrap(), body.len());
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


struct WireMotion {
    start_pos: (u16, u16, u16),
    start_time: u16,
    end_pos: (u16, u16, u16),
    end_time: u16,
}

impl Struct for WireMotion {
    fn read_from<R: Reader>(r: &mut R) -> IoResult<WireMotion> {
        let start_pos = try!(Struct::read_from(r));
        let start_time = try!(Struct::read_from(r));
        let end_pos = try!(Struct::read_from(r));
        let end_time = try!(Struct::read_from(r));
        Ok(WireMotion {
            start_pos: start_pos,
            start_time: start_time,
            end_pos: end_pos,
            end_time: end_time,
        })
    }

    fn write_to<W: Writer>(&self, w: &mut W) -> IoResult<()> {
        (self.start_pos, self.start_time, self.end_pos, self.end_time).write_to(w)
    }

    fn size(&self) -> uint {
        (self.start_pos, self.start_time, self.end_pos, self.end_time).size()
    }
}



trait Struct {
    fn read_from<R: Reader>(r: &mut R) -> IoResult<Self>;
    fn write_to<W: Writer>(&self, w: &mut W) -> IoResult<()>;
    fn size(&self) -> uint;

    fn decode_from(buf: &[u8]) -> IoResult<Self> {
        Struct::read_from(&mut BufReader::new(buf))
    }

    fn encode_to(&self, buf: &mut [u8]) -> IoResult<()> {
        self.write_to(&mut BufWriter::new(buf))
    }
}

impl Struct for u16 {
    fn read_from<R: Reader>(r: &mut R) -> IoResult<u16> {
        r.read_le_u16()
    }

    fn write_to<W: Writer>(&self, w: &mut W) -> IoResult<()> {
        w.write_le_u16(*self)
    }

    fn size(&self) -> uint { mem::size_of::<u16>() }
}

impl<A: Struct, B: Struct> Struct for (A, B) {
    fn read_from<R: Reader>(r: &mut R) -> IoResult<(A, B)> {
        let a = try!(Struct::read_from(r));
        let b = try!(Struct::read_from(r));
        Ok((a, b))
    }

    fn write_to<W: Writer>(&self, w: &mut W) -> IoResult<()> {
        let (ref a, ref b) = *self;
        try!(a.write_to(w));
        try!(b.write_to(w));
        Ok(())
    }

    fn size(&self) -> uint {
        let (ref a, ref b) = *self;
        a.size() + b.size()
    }
}

impl<A: Struct, B: Struct, C: Struct> Struct for (A, B, C) {
    fn read_from<R: Reader>(r: &mut R) -> IoResult<(A, B, C)> {
        let a = try!(Struct::read_from(r));
        let b = try!(Struct::read_from(r));
        let c = try!(Struct::read_from(r));
        Ok((a, b, c))
    }

    fn write_to<W: Writer>(&self, w: &mut W) -> IoResult<()> {
        let (ref a, ref b, ref c) = *self;
        try!(a.write_to(w));
        try!(b.write_to(w));
        try!(c.write_to(w));
        Ok(())
    }

    fn size(&self) -> uint {
        let (ref a, ref b, ref c) = *self;
        a.size() + b.size() + c.size()
    }
}

impl<A: Struct, B: Struct, C: Struct, D: Struct> Struct for (A, B, C, D) {
    fn read_from<R: Reader>(r: &mut R) -> IoResult<(A, B, C, D)> {
        let a = try!(Struct::read_from(r));
        let b = try!(Struct::read_from(r));
        let c = try!(Struct::read_from(r));
        let d = try!(Struct::read_from(r));
        Ok((a, b, c, d))
    }

    fn write_to<W: Writer>(&self, w: &mut W) -> IoResult<()> {
        let (ref a, ref b, ref c, ref d) = *self;
        try!(a.write_to(w));
        try!(b.write_to(w));
        try!(c.write_to(w));
        try!(d.write_to(w));
        Ok(())
    }

    fn size(&self) -> uint {
        let (ref a, ref b, ref c, ref d) = *self;
        a.size() + b.size() + c.size() + d.size()
    }
}
