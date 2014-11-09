#![crate_name = "backend"]
#![feature(globs)]
#![feature(phase)]
#![feature(tuple_indexing, if_let)]
#![feature(unboxed_closures, overloaded_calls)]
#![feature(macro_rules)]
#![feature(associated_types)]
#![allow(non_upper_case_globals)]

#[phase(plugin, link)]
extern crate log;
extern crate time;

extern crate physics;

use std::cmp;
use std::collections::HashMap;
use std::io;
use std::io::{BufReader, BufWriter};
use std::io::IoResult;
use std::mem;
use std::rand::{StdRng, Rng};
use std::u16;

use physics::{CHUNK_SIZE, CHUNK_BITS, TILE_SIZE};
use physics::v3::{V3, scalar};

use timer::WakeQueue;
use wire::{WireReader, WireWriter};
use msg::Motion as WireMotion;
use msg::{Request, Response};
use state::InputBits;

use types::{LocalTime, LocalCoord, Time, ClientId, EntityId, ToGlobal, ToLocal};

mod msg;
mod wire;
mod tasks;
mod state;
mod timer;
mod types;

fn main() {
    let (req_send, req_recv) = channel();
    let (resp_send, resp_recv) = channel();

    spawn(proc() {
        let reader = io::stdin();
        tasks::run_input(reader, req_send).unwrap();
    });

    spawn(proc() {
        let writer = io::BufferedWriter::new(io::stdout().unwrap());
        tasks::run_output(writer, resp_recv).unwrap();
    });

    let mut state = state::State::new();
    state.init_terrain();
    let mut server = Server::new(resp_send, state);
    server.run(req_recv);
}


pub enum WakeReason {
    HandleInput(ClientId, InputBits),
    PhysicsUpdate(ClientId),
}

struct Server {
    resps: Sender<(ClientId, Response)>,
    state: state::State,
    wake_queue: WakeQueue<WakeReason>,
}

impl Server {
    fn new(resps: Sender<(ClientId, Response)>,
           state: state::State) -> Server {
        Server {
            resps: resps,
            state: state,
            wake_queue: WakeQueue::new(),
        }
    }

    fn run(&mut self, reqs: Receiver<(ClientId, Request)>) {
        loop {
            let wake_recv = self.wake_queue.wait_recv(now());

            select! {
                () = wake_recv.recv() => {
                    let now = now();
                    while let Some((time, reason)) = self.wake_queue.pop(now) {
                        self.handle_wake(time, reason);
                    }
                },

                (id, req) = reqs.recv() => {
                    self.handle_req(now(), id, req);
                }
            }
        }
    }

    fn handle_req(&mut self,
                  now: Time,
                  client_id: ClientId,
                  req: Request) {
        match req {
            msg::GetTerrain => {
                warn!("client {} used deprecated opcode GetTerrain", client_id);
            },

            msg::UpdateMotion(wire_motion) => {
                warn!("client {} used deprecated opcode UpdateMotion", client_id);
            },

            msg::Ping(cookie) => {
                self.resps.send((client_id, msg::Pong(cookie, now.to_local())));
            },

            msg::Input(time, input) => {
                let time = cmp::max(time.to_global(now), now);
                let input = InputBits::from_bits_truncate(input);
                self.wake_queue.push(time, HandleInput(client_id, input));
            },

            msg::Login(secret, name) => {
                log!(10, "login request for {}", name);
                self.state.add_client(now, client_id);

                let info = msg::InitData {
                    entity_id: client_id as EntityId,
                    camera_pos: (0, 0),
                    chunks: 8 * 8,
                    entities: 1,
                };
                self.resps.send((client_id, msg::Init(info)));

                for c in range(0, 8 * 8) {
                    let data = self.state.get_terrain_rle16(c);
                    self.resps.send((client_id, msg::TerrainChunk(c as u16, data)));
                }

                let ce = self.state.client_entity(client_id).unwrap();
                let motion = entity_motion(now, ce);
                let anim = ce.entity.anim;
                self.resps.send((client_id, msg::EntityUpdate(ce.client.entity_id, motion, anim)));
            },

            msg::AddClient => {
            },

            msg::RemoveClient => {
                self.state.remove_client(client_id);
                self.resps.send((client_id, msg::ClientRemoved));
            },

            msg::BadMessage(opcode) => {
                warn!("unrecognized opcode from client {}: {:x}",
                      client_id, opcode.unwrap());
            },
        }
    }

    fn handle_wake(&mut self,
                   now: Time,
                   reason: WakeReason) {
        match reason {
            HandleInput(client_id, input) => {
                let updated = self.state.update_input(now, client_id, input);
                let (entity_id, motion, anim) = {
                    let ce = self.state.client_entity(client_id).unwrap();
                    (ce.client.entity_id,
                     entity_motion(now, ce),
                     ce.entity.anim)
                };
                for &send_id in self.state.clients.keys() {
                    self.resps.send((send_id, msg::EntityUpdate(entity_id, motion, anim)));
                }
            },

            PhysicsUpdate(client_id) => {
            },
        }
    }
}

fn now() -> Time {
    let timespec = time::get_time();
    (timespec.sec as Time * 1000) + (timespec.nsec / 1000000) as Time
}

fn entity_motion(now: Time, ce: state::ClientEntity) -> WireMotion {
    let pos = ce.entity.pos(now);
    let world_base = state::base_chunk(pos);
    let local_base = state::offset_base_chunk(world_base, ce.client.chunk_offset);

    let start_pos = state::world_to_local(ce.entity.start_pos, world_base, local_base);
    let end_pos = state::world_to_local(ce.entity.end_pos, world_base, local_base);
    
    WireMotion {
        start_time: ce.entity.start_time.to_local(),
        end_time: (ce.entity.start_time + ce.entity.duration as Time).to_local(),
        start_pos: (start_pos.x as u16,
                    start_pos.y as u16,
                    start_pos.z as u16),
        end_pos: (end_pos.x as u16,
                    end_pos.y as u16,
                    end_pos.z as u16),
    }
}
