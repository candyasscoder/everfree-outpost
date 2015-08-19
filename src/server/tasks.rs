use std::io::{self, Read, Write};
use std::sync::mpsc::{Sender, Receiver};

use msg::{Request, Response};
use wire::{WireReader, WireWriter};
use types::WireId;

pub fn run_input<R: Read>(r: R, send: Sender<(WireId, Request)>) -> io::Result<()> {
    let mut wr = WireReader::new(r);
    loop {
        match Request::read_from(&mut wr) {
            Ok((id, req)) => send.send((id, req)).unwrap(),
            Err(e) => {
                use std::io::ErrorKind::*;
                warn!("error reading message from wire: {}", e);
                match e.kind() {
                    NotFound |
                    PermissionDenied |
                    ConnectionRefused |
                    ConnectionReset |
                    ConnectionAborted |
                    NotConnected |
                    BrokenPipe => return Err(e),
                    _ => {},
                }
            }
        }
    }
}

pub fn run_output<W: Write>(w: W, recv: Receiver<(WireId, Response)>) -> io::Result<()> {
    let mut ww = WireWriter::new(w);
    loop {
        let (id, req) = recv.recv().unwrap();
        try!(req.write_to(id, &mut ww));
    }
}
