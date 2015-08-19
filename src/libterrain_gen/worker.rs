use std::sync::mpsc::{Sender, Receiver};
use rand::SeedableRng;
use time;

use libserver_types::*;
use libserver_config::Data;
use libserver_config::Storage;

use GenChunk;
use forest::Provider as ForestProvider;


pub enum Command {
    Generate(Stable<PlaneId>, V2),
}

pub type Response = (Stable<PlaneId>, V2, GenChunk);

pub fn run(data: &Data,
           storage: &Storage,
           recv: Receiver<Command>,
           send: Sender<Response>) {
    let mut w = Worker::new(data, storage);

    for cmd in recv.iter() {
        use self::Command::*;
        match cmd {
            Generate(pid, cpos) => {
                let gc = w.generate_forest_chunk(pid, cpos);
                send.send((pid, cpos, gc)).unwrap();
            },
        }
    }
}


struct Worker<'d> {
    forest: ForestProvider<'d>,
}

impl<'d> Worker<'d> {
    fn new(data: &'d Data, storage: &'d Storage) -> Worker<'d> {
        let rng = SeedableRng::from_seed([0xe0e0e0e0,
                                          0x00012345,
                                          0xe0e0e0e0,
                                          0x00012345]);

        Worker {
            forest: ForestProvider::new(data, storage, rng),
        }
    }

    pub fn generate_forest_chunk(&mut self, pid: Stable<PlaneId>, cpos: V2) -> GenChunk {
        let start = now();
        let gc = self.forest.generate(pid, cpos);
        let end = now();
        info!("generated {} {:?} in {} ms", pid.unwrap(), cpos, end - start);
        gc
    }
}


fn now() -> u64 {
    let timespec = time::get_time();
    (timespec.sec as u64 * 1000) + (timespec.nsec / 1000000) as u64
}
