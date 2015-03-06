use std::old_io::IoResult;
use std::sync::mpsc::{Sender, Receiver};

use msg::{Request, Response};
use wire::{WireReader, WireWriter};
use types::WireId;

pub fn run_input<R: Reader>(r: R, send: Sender<(WireId, Request)>) -> IoResult<()> {
    let mut wr = WireReader::new(r);
    loop {
        match Request::read_from(&mut wr) {
            Ok((id, req)) => send.send((id, req)).unwrap(),
            Err(e) => {
                use std::old_io::IoErrorKind::*;
                warn!("error reading message from wire: {}", e);
                match e.kind {
                    EndOfFile |
                    ConnectionFailed |
                    Closed |
                    ConnectionReset |
                    ConnectionAborted |
                    NotConnected |
                    BrokenPipe |
                    ResourceUnavailable |
                    NoProgress => return Err(e),
                    _ => {},
                }
            }
        }
    }
}

pub fn run_output<W: Writer>(w: W, recv: Receiver<(WireId, Response)>) -> IoResult<()> {
    let mut ww = WireWriter::new(w);
    loop {
        let (id, req) = recv.recv().unwrap();
        try!(req.write_to(id, &mut ww));
    }
}
