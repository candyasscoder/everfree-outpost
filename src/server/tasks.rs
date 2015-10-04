//! Small functions that need to run on background threads.  Currently this just means
//! serialization and deserialization of requests/responses.
//!
//! De/serialization is actually pretty fast, but the input side does need to run in a separate
//! thread so the main `Engine` loop can `select` over a channel of incoming `Request`s along with
//! the channels for other types of events.

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
