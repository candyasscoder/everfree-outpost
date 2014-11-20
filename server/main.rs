#![crate_name = "backend"]
#![feature(globs)]
#![feature(phase)]
#![feature(tuple_indexing, if_let)]
#![feature(unboxed_closures)]
#![feature(macro_rules)]
#![feature(associated_types)]
#![allow(non_upper_case_globals)]

#[phase(plugin, link)]
extern crate log;
extern crate time;
extern crate serialize;

extern crate physics;

use std::cmp;
use std::io;

use physics::v3::V3;

use timer::WakeQueue;
use msg::Motion as WireMotion;
use msg::{Request, Response};
use input::InputBits;
use state::LOCAL_SIZE;
use block_data::BlockData;

use types::{Time, ToGlobal, ToLocal};
use types::{ClientId, EntityId};


mod msg;
mod wire;
mod tasks;
mod state;
mod timer;
mod types;
mod view;
mod input;
mod gen;
mod block_data;

fn main() {
    let block_data = {
        use std::os;
        use std::io::fs::File;
        use serialize::json;

        let path = &os::args()[1];
        log!(10, "reading block data from {}", path);
        let mut file = File::open(&Path::new(path)).unwrap();
        let json = json::from_reader(&mut file).unwrap();
        BlockData::from_json(json).unwrap()
    };

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

    let mut state = state::State::new(block_data);
    let start = now();
    state.init_terrain();
    let end = now();
    log!(10, "generated terrain in {} msec", end - start);
    let mut server = Server::new(resp_send, state);
    server.run(req_recv);
}


pub enum WakeReason {
    HandleInput(ClientId, InputBits),
    PhysicsUpdate(ClientId),
    CheckView(ClientId),
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
            Request::GetTerrain => {
                warn!("client {} used deprecated opcode GetTerrain", client_id);
            },

            Request::UpdateMotion(_wire_motion) => {
                warn!("client {} used deprecated opcode UpdateMotion", client_id);
            },

            Request::Ping(cookie) => {
                self.resps.send((client_id, Response::Pong(cookie, now.to_local())));
            },

            Request::Input(time, input) => {
                let time = cmp::max(time.to_global(now), now);
                let input = InputBits::from_bits_truncate(input);
                self.wake_queue.push(time, WakeReason::HandleInput(client_id, input));
            },

            Request::Login(_secret, name) => {
                log!(10, "login request for {}", name);
                self.state.add_client(now, client_id);

                let info = msg::InitData {
                    entity_id: client_id as EntityId,
                    camera_pos: (0, 0),
                    chunks: 8 * 8,
                    entities: 1,
                };
                self.resps.send((client_id, Response::Init(info)));

                let (region, offset) = {
                    let ce = self.state.client_entity(client_id).unwrap();
                    let motion = entity_motion(now, ce);
                    let anim = ce.entity.anim;
                    self.resps.send((client_id,
                                     Response::EntityUpdate(ce.client.entity_id, motion, anim)));
                    log!(10, "pos={}, region={}",
                         ce.entity.pos(now),
                         ce.client.view_state.region());

                    (ce.client.view_state.region(),
                     chunk_offset(ce.entity.pos(now), ce.client.chunk_offset))
                };

                for (x,y) in region.points() {
                    self.load_chunk(client_id, x, y, offset);
                }
                self.wake_queue.push(now + 1000, WakeReason::CheckView(client_id));
            },

            Request::AddClient => {
            },

            Request::RemoveClient => {
                self.state.remove_client(client_id);
                self.resps.send((client_id, Response::ClientRemoved));
            },

            Request::BadMessage(opcode) => {
                warn!("unrecognized opcode from client {}: {:x}",
                      client_id, opcode.unwrap());
            },
        }
    }

    fn handle_wake(&mut self,
                   now: Time,
                   reason: WakeReason) {
        match reason {
            WakeReason::HandleInput(client_id, input) => {
                let updated = self.state.update_input(now, client_id, input);
                if updated {
                    self.post_physics_update(now, client_id);
                }
            },

            WakeReason::PhysicsUpdate(client_id) => {
                let updated = self.state.update_physics(now, client_id);
                if updated {
                    self.post_physics_update(now, client_id);
                }
            },

            WakeReason::CheckView(client_id) => {
                let (result, offset) = {
                    let ce = match self.state.client_entity_mut(client_id) {
                        Some(ce) => ce,
                        None => return,
                    };
                    let pos = ce.entity.pos(now);
                    (ce.client.view_state.update(pos + V3::new(16, 16, 0)),
                     chunk_offset(pos, ce.client.chunk_offset))
                };


                if let Some((old_region, new_region)) = result {
                    for (x,y) in old_region.points().filter(|&(x,y)| !new_region.contains(x,y)) {
                        self.unload_chunk(client_id, x, y, offset);
                    }

                    for (x,y) in new_region.points().filter(|&(x,y)| !old_region.contains(x,y)) {
                        self.load_chunk(client_id, x, y, offset);
                    }
                }

                self.wake_queue.push(now + 1000, WakeReason::CheckView(client_id));
            },
        }
    }

    fn post_physics_update(&mut self,
                           now: Time,
                           client_id: ClientId) {
        let (entity_id, motion, anim, end_time) = {
            let ce = self.state.client_entity(client_id).unwrap();
            (ce.client.entity_id,
             entity_motion(now, ce),
             ce.entity.anim,
             ce.entity.end_time())
        };
        for &send_id in self.state.clients.keys() {
            self.resps.send((send_id, Response::EntityUpdate(entity_id, motion, anim)));
        }

        if motion.start_pos != motion.end_pos {
            self.wake_queue.push(end_time, WakeReason::PhysicsUpdate(client_id));
        }
    }

    fn load_chunk(&self,
                  client_id: ClientId,
                  x: i32, y: i32,
                  offset: V3) {
        let cx = (x + offset.x) & (LOCAL_SIZE - 1);
        let cy = (y + offset.y) & (LOCAL_SIZE - 1);

        log!(10, "load {},{} as {},{} for {}", x, y, cx, cy, client_id);

        let idx = cy * LOCAL_SIZE + cx;
        let data = self.state.get_terrain_rle16(idx as uint);
        self.resps.send((client_id, Response::TerrainChunk(idx as u16, data)));
    }

    fn unload_chunk(&self,
                    client_id: ClientId,
                    x: i32, y: i32,
                    offset: V3) {
        let cx = (x + offset.x) & (LOCAL_SIZE - 1);
        let cy = (y + offset.y) & (LOCAL_SIZE - 1);

        log!(10, "unload {},{} as {},{} for {}", x, y, cx, cy, client_id);

        let idx = cy * LOCAL_SIZE + cx;
        self.resps.send((client_id, Response::UnloadChunk(idx as u16)));
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

fn chunk_offset(pos: V3, extra_offset: (u8, u8)) -> V3 {
    let world_base = state::base_chunk(pos);
    let local_base = state::offset_base_chunk(world_base, extra_offset);
    local_base - world_base
}
