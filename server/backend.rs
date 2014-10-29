#![feature(phase)]
#![feature(macro_rules)]
#![allow(non_upper_case_globals)]

#[phase(plugin, link)]
extern crate log;

extern crate physics;

use std::collections::HashSet;
use std::io;
use std::io::IoResult;
use std::rand::{StdRng, Rng};

use physics::CHUNK_BITS;

fn main() {
    real_main().unwrap()
}


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

    TerrainChunk = 0x8001,

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


type Chunk = [u16, ..1 << (3 * CHUNK_BITS)];

const LOCAL_BITS: uint = 3;

struct State {
    map: [Chunk, ..1 << (2 * LOCAL_BITS)],
    clients: HashSet<u16>,
}


fn real_main() -> IoResult<()> {
    let mut stdin = io::stdin();
    let mut stdout = io::BufferedWriter::new(io::stdout().unwrap());

    let mut rng = try!(StdRng::new());

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
            AddClient => {},
            RemoveClient => {
                try!(write_msg(&mut stdout, id, ClientRemoved, &[]));
            },
            _ => {
                warning!("unrecognized opcode from client {}: {:x} ({} bytes)",
                         id, opcode.unwrap(), body.len());
            },
        }
    }
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
