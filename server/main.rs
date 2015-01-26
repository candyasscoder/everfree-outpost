#![crate_name = "backend"]
#![feature(unboxed_closures)]
#![feature(unsafe_destructor)]
#![allow(non_upper_case_globals)]
#![allow(unstable)]
#![allow(dead_code)]

#[macro_use] extern crate bitflags;
#[macro_use] extern crate log;
extern crate libc;
extern crate time;
extern crate serialize;

extern crate collect;

extern crate physics;

use std::cmp;
use std::error::Error;
use std::io;
use std::sync::mpsc::{Sender, Receiver, channel};
use std::thread::Thread;
use serialize::json;

use physics::v3::V3;

use timer::WakeQueue;
use msg::Motion as WireMotion;
use msg::{Request, Response};
use input::{InputBits, ActionBits};
use state::LOCAL_SIZE;
use state::StateChange::ChunkUpdate;
use data::Data;
use world::object::{ObjectRefBase, ClientRef};

use types::{Time, ToGlobal, ToLocal};
use types::{ClientId, EntityId};


#[macro_use] mod util;
mod msg;
mod wire;
mod tasks;
mod state;
mod timer;
mod types;
mod view;
mod input;
mod gen;
mod data;
mod lua;
mod script;
mod world;
mod terrain2;


fn read_json(path: &str) -> json::Json {
    use std::io::fs::File;
    let mut file = File::open(&Path::new(path)).unwrap();
    let json = json::from_reader(&mut file).unwrap();
    json
}

fn main() {
    let (data, script_path) = {
        use std::os;

        let block_path = &os::args()[1];
        log!(10, "reading block data from {}", block_path);
        let block_json = read_json(block_path.as_slice());

        let template_path = &os::args()[2];
        log!(10, "reading template data from {}", template_path);
        let template_json = read_json(template_path.as_slice());

        let data = Data::from_json(block_json, template_json).unwrap();

        let script_path = os::args()[3].clone();
        (data, script_path)
    };

    let (req_send, req_recv) = channel();
    let (resp_send, resp_recv) = channel();

    Thread::spawn(move || {
        let reader = io::stdin();
        tasks::run_input(reader, req_send).unwrap();
    });

    Thread::spawn(move || {
        let writer = io::BufferedWriter::new(io::stdout());
        tasks::run_output(writer, resp_recv).unwrap();
    });

    let state = state::State::new(&data, &*script_path);
    let mut server = Server::new(resp_send, state);
    server.run(req_recv);
}


#[derive(Copy)]
pub enum WakeReason {
    HandleInput(ClientId, InputBits),
    HandleAction(ClientId, ActionBits),
    PhysicsUpdate(ClientId),
    CheckView(ClientId),
}

struct Server<'a> {
    resps: Sender<(ClientId, Response)>,
    state: state::State<'a>,
    wake_queue: WakeQueue<WakeReason>,
}

impl<'a> Server<'a> {
    fn new(resps: Sender<(ClientId, Response)>,
           state: state::State<'a>) -> Server<'a> {
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
                wake = wake_recv.recv() => {
                    let () = wake.unwrap();
                    let now = now();
                    while let Some((time, reason)) = self.wake_queue.pop(now) {
                        self.handle_wake(time, reason);
                    }
                },

                req = reqs.recv() => {
                    let (id, req) = req.unwrap();
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
                warn!("client {} used deprecated opcode GetTerrain", client_id.unwrap());
            },

            Request::UpdateMotion(_wire_motion) => {
                warn!("client {} used deprecated opcode UpdateMotion", client_id.unwrap());
            },

            Request::Ping(cookie) => {
                self.resps.send((client_id, Response::Pong(cookie, now.to_local())))
                    .unwrap();
            },

            Request::Input(time, input) => {
                let time = cmp::max(time.to_global(now), now);
                let input = InputBits::from_bits_truncate(input);
                self.wake_queue.push(time, WakeReason::HandleInput(client_id, input));
            },

            Request::Login(_secret, name) => {
                log!(10, "login request for {}", name);

                let (region, offset, pawn_id) = {
                    let mut client = self.state.add_client(now, client_id);
                    info!("client connected with id {:?} <-> {:?}", client.id(), client_id);

                    let pawn = client.pawn().unwrap();
                    let motion = entity_motion(now, pawn.motion(), client.chunk_offset());
                    let anim = pawn.anim();
                    self.resps.send((client_id,
                                     Response::EntityUpdate(pawn.id(), motion, anim)))
                        .unwrap();
                    log!(10, "pos={:?}, region={:?}",
                         pawn.pos(now),
                         client.view_state().region());

                    (client.view_state().region(),
                     chunk_offset(pawn.pos(now), client.chunk_offset()),
                     pawn.id())
                };

                let info = msg::InitData {
                    entity_id: pawn_id,
                    camera_pos: (0, 0),
                    chunks: 8 * 8,
                    entities: 1,
                };
                self.resps.send((client_id, Response::Init(info))).unwrap();

                for (x,y) in region.points() {
                    self.load_chunk(client_id, x, y, offset);
                }
                self.wake_queue.push(now + 1000, WakeReason::CheckView(client_id));
            },

            Request::Action(time, action) => {
                let time = cmp::max(time.to_global(now), now);
                let action = ActionBits::from_bits_truncate(action);
                self.wake_queue.push(time, WakeReason::HandleAction(client_id, action));
            },

            Request::AddClient => {
            },

            Request::RemoveClient => {
                self.state.remove_client(client_id);
                self.resps.send((client_id, Response::ClientRemoved)).unwrap();
            },

            Request::BadMessage(opcode) => {
                warn!("unrecognized opcode from client {:?}: {:x}",
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
                match updated {
                    Ok(true) => self.post_physics_update(now, client_id),
                    Ok(false) => {},
                    Err(e) => warn!("update_input error: {}", e.description()),
                }
            },

            WakeReason::HandleAction(client_id, action) => {
                let updates = self.state.perform_action(now, client_id, action);
                for update in updates.into_iter() {
                    match update {
                        ChunkUpdate(cx, cy) => {
                            for c in self.state.world().clients() {
                                if !c.view_state().region().contains(cx, cy) {
                                    continue;
                                }

                                let offset = chunk_offset(c.pawn().unwrap().pos(now),
                                                          c.chunk_offset());
                                let idx = chunk_to_idx(cx, cy, offset);
                                let data = self.state.get_terrain_rle16(cx, cy);
                                self.resps.send((c.id(), Response::TerrainChunk(idx as u16, data)))
                                    .unwrap();
                            }
                        },
                    }
                }
            },

            WakeReason::PhysicsUpdate(client_id) => {
                let updated = self.state.update_physics(now, client_id);
                match updated {
                    Ok(true) => self.post_physics_update(now, client_id),
                    Ok(false) => {},
                    Err(e) => warn!("update_physics error: {}", e.description()),
                }
            },

            WakeReason::CheckView(client_id) => {
                let (result, offset) = {
                    let mut client = match self.state.world_mut().get_client_mut(client_id) {
                        Some(x) => x,
                        None => return,
                    };
                    let pos = client.pawn().unwrap().pos(now);
                    (client.view_state_mut().update(pos + V3::new(16, 16, 0)),
                     chunk_offset(pos, client.chunk_offset()))
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

        drop(self.state.world_mut().take_journal());
    }

    fn post_physics_update(&mut self,
                           now: Time,
                           client_id: ClientId) {
        let (entity_id, motion, anim, end_time) = {
            let client = self.state.world().client(client_id);
            let entity = client.pawn().unwrap();
            (entity.id(),
             entity_motion(now, entity.motion(), client.chunk_offset()),
             entity.anim(),
             entity.motion().end_time())
        };
        for client in self.state.world().clients() {
            let send_id = client.id();
            self.resps.send((send_id, Response::EntityUpdate(entity_id, motion.clone(), anim)))
                .unwrap();
        }

        if motion.start_pos != motion.end_pos {
            self.wake_queue.push(end_time, WakeReason::PhysicsUpdate(client_id));
        }
    }

    fn load_chunk(&mut self,
                  client_id: ClientId,
                  cx: i32, cy: i32,
                  offset: V3) {
        self.state.load_chunk(cx, cy);

        let idx = chunk_to_idx(cx, cy, offset);
        let data = self.state.get_terrain_rle16(cx, cy);
        self.resps.send((client_id, Response::TerrainChunk(idx as u16, data)))
            .unwrap();
    }

    fn unload_chunk(&mut self,
                    client_id: ClientId,
                    cx: i32, cy: i32,
                    offset: V3) {
        self.state.unload_chunk(cx, cy);

        let idx = chunk_to_idx(cx, cy, offset);
        self.resps.send((client_id, Response::UnloadChunk(idx as u16)))
            .unwrap();
    }
}

fn chunk_to_idx(cx: i32, cy: i32, offset: V3) -> i32 {
    let lx = (cx + offset.x) & (LOCAL_SIZE - 1);
    let ly = (cy + offset.y) & (LOCAL_SIZE - 1);
    ly * LOCAL_SIZE + lx
}

fn now() -> Time {
    let timespec = time::get_time();
    (timespec.sec as Time * 1000) + (timespec.nsec / 1000000) as Time
}

fn entity_motion(now: Time,
                 motion: &world::Motion,
                 client_offset: (u8, u8)) -> WireMotion {
    let pos = motion.pos(now);
    let world_base = state::base_chunk(pos);
    let local_base = state::offset_base_chunk(world_base, client_offset);

    let start_pos = state::world_to_local(motion.start_pos, world_base, local_base);
    let end_pos = state::world_to_local(motion.end_pos, world_base, local_base);
    
    WireMotion {
        start_time: motion.start_time.to_local(),
        end_time: (motion.start_time + motion.duration as Time).to_local(),
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
