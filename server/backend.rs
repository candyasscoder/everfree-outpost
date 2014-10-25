use std::io;
use std::io::IoResult;

fn main() {
    real_main().unwrap()
}

fn real_main() -> IoResult<()> {
    let mut stdin = io::stdin();
    let mut stdout = io::BufferedWriter::new(io::stdout().unwrap());
    let mut stderr = io::stderr().unwrap();

    try!(stderr.write_str("start\n"));

    loop {
        let id = try!(stdin.read_le_u16());
        let size = try!(stdin.read_le_u16());
        let opcode = try!(stdin.read_le_u16());
        let body = try!(stdin.read_exact(size as uint - 2));

        match opcode {
            0xff00 => {
                try!(stderr.write_str(format!("add client: {}\n", id).as_slice()));
            },
            0xff01 => {
                try!(stderr.write_str(format!("remove client: {}\n", id).as_slice()));
                try!(stdout.write_le_u16(id));
                try!(stdout.write_le_u16(2));
                try!(stdout.write_le_u16(0xff02));
                try!(stdout.flush());
            },
            _ => {
                try!(stderr.write_str(format!(
                            "echo message [{}]: {:x} ({} bytes)\n", id, opcode, size - 2).as_slice()));
                try!(stdout.write_le_u16(id));
                try!(stdout.write_le_u16(size));
                try!(stdout.write_le_u16(opcode));
                try!(stdout.write(body.as_slice()));
                try!(stdout.flush());
            },
        }
    }
}
