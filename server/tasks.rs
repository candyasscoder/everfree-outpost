use std::io::IoResult;
use std::sync::mpsc::{Sender, Receiver};

use msg::{Request, Response};
use wire::{WireReader, WireWriter};
use types::ClientId;

pub fn run_input<R: Reader>(r: R, send: Sender<(ClientId, Request)>) -> IoResult<()> {
    let mut wr = WireReader::new(r);
    loop {
        let (id, req) = try!(Request::read_from(&mut wr));
        send.send((id, req)).unwrap();
    }
}

pub fn run_output<W: Writer>(w: W, recv: Receiver<(ClientId, Response)>) -> IoResult<()> {
    let mut ww = WireWriter::new(w);
    loop {
        let (id, req) = recv.recv().unwrap();
        try!(req.write_to(id, &mut ww));
    }
}
