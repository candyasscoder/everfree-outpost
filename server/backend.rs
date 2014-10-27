#![feature(macro_rules)]
#![allow(non_upper_case_globals)]
use std::io;
use std::io::IoResult;
use std::rand::{StdRng, Rng};

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


fn real_main() -> IoResult<()> {
    let mut stdin = io::stdin();
    let mut stdout = io::BufferedWriter::new(io::stdout().unwrap());
    let mut stderr = io::stderr().unwrap();

    let write_msg = |client: u16, opcode: Opcode, data: &[u8]| {
        try!(stdout.write_le_u16(client));
        try!(stdout.write_le_u16(2 + data.len() as u16));
        try!(stdout.write_le_u16(opcode.unwrap()));
        try!(stdout.write(data));
        try!(stdout.flush());
        Ok(())
    };

    try!(stderr.write_str("start\n"));
    let mut rng = try!(StdRng::new());

    loop {
        let id = try!(stdin.read_le_u16());
        let size = try!(stdin.read_le_u16());
        let opcode = try!(stdin.read_le_u16());
        let body = try!(stdin.read_exact(size as uint - 2));

        match Opcode(opcode) {
            GetTerrain => {
                for c in range(0, 8 * 8) {
                    let mut data = Vec::from_elem(1 + 16 * 16 * 16, 0u16);
                    data[0] = c;
                    for i in range(0, 16 * 16) {
                        if rng.gen_range(0, 10) == 0u8 {
                            data[1 + i] = 0;
                        } else {
                            data[1 + i] = 1;
                        }
                    }
                    try!(write_msg(id, TerrainChunk, convert(data.as_slice())));
                }
            },
            AddClient => {
                try!(stderr.write_str(format!("add client: {}\n", id).as_slice()));
            },
            RemoveClient => {
                try!(stderr.write_str(format!("remove client: {}\n", id).as_slice()));
                try!(write_msg(id, ClientRemoved, &[]));
            },
            opcode => {
                try!(stderr.write_str(format!(
                            "echo message [{}]: {:x} ({} bytes)\n", id, opcode.unwrap(), size - 2).as_slice()));
                try!(write_msg(id, opcode, body.as_slice()));
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
