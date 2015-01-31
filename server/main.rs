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
use std::collections::HashMap;
use std::error::Error;
use std::io;
use std::sync::mpsc::{Sender, Receiver, channel};
use std::thread::Thread;
use serialize::json;

use physics::v3::{Vn, V3, V2, scalar};

use timer::WakeQueue;
use msg::Motion as WireMotion;
use msg::{Request, Response};
use input::{InputBits, ActionBits};
use state::LOCAL_SIZE;
use state::StateChange::ChunkUpdate;
use data::Data;
use world::object::{ObjectRef, ObjectRefBase, ClientRef};

use types::{Time, ToGlobal, ToLocal};
use types::{WireId, ClientId, EntityId};


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
    PhysicsUpdate(EntityId),
    CheckView(ClientId),
}

struct Server<'a> {
    resps: Sender<(WireId, Response)>,
    state: state::State<'a>,
    wake_queue: WakeQueue<WakeReason>,
    wire_id_map: HashMap<WireId, ClientId>,
}

impl<'a> Server<'a> {
    fn new(resps: Sender<(WireId, Response)>,
           state: state::State<'a>) -> Server<'a> {
        Server {
            resps: resps,
            state: state,
            wake_queue: WakeQueue::new(),
            wire_id_map: HashMap::new(),
        }
    }

    fn run(&mut self, reqs: Receiver<(WireId, Request)>) {
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

    fn wire_to_client(&self, wire_id: WireId) -> Option<ClientId> {
        self.wire_id_map.get(&wire_id).map(|&x| x)
    }

    fn handle_req(&mut self,
                  now: Time,
                  wire_id: WireId,
                  req: Request) {
        match req {
            Request::GetTerrain => {
                warn!("connection {} used deprecated opcode GetTerrain", wire_id.unwrap());
            },

            Request::UpdateMotion(_wire_motion) => {
                warn!("connection {} used deprecated opcode UpdateMotion", wire_id.unwrap());
            },

            Request::Ping(cookie) => {
                self.resps.send((wire_id, Response::Pong(cookie, now.to_local())))
                    .unwrap();
            },

            Request::Input(time, input) => {
                let client_id = unwrap_or!(self.wire_to_client(wire_id));
                let time = cmp::max(time.to_global(now), now);
                let input = InputBits::from_bits_truncate(input);
                self.wake_queue.push(time, WakeReason::HandleInput(client_id, input));
            },

            Request::Login(_secret, name) => {
                log!(10, "login request for {}", name);

                let (cid, eid) = {
                    let client = self.state.add_client(now, wire_id);
                    (client.id(), client.pawn_id())
                };
                self.wire_id_map.insert(wire_id, cid);
                self.reset_viewport(now, cid, true);
                if let Some(eid) = eid {
                    self.update_entity_motion(now, eid);
                }
                self.wake_queue.push(now + 1000, WakeReason::CheckView(cid));
            },

            Request::Action(time, action) => {
                let client_id = unwrap_or!(self.wire_to_client(wire_id));
                let time = cmp::max(time.to_global(now), now);
                let action = ActionBits::from_bits_truncate(action);
                self.wake_queue.push(time, WakeReason::HandleAction(client_id, action));
            },

            Request::AddClient => {
            },

            Request::RemoveClient => {
                // TODO: need to unload chunks visible to this client
                if let Some(client_id) = self.wire_to_client(wire_id) {
                    use std::io::fs::File;
                    let file = File::create(&Path::new("save.client")).unwrap();
                    let mut sw = world::save::SaveWriter::new(file);
                    sw.save_client(&self.state.world().client(client_id)).unwrap();

                    self.state.remove_client(client_id);
                    self.wire_id_map.remove(&wire_id);
                }
                self.resps.send((wire_id, Response::ClientRemoved)).unwrap();
            },

            Request::BadMessage(opcode) => {
                warn!("unrecognized opcode from connection {:?}: {:x}",
                      wire_id, opcode.unwrap());
            },
        }
    }

    fn handle_wake(&mut self,
                   now: Time,
                   reason: WakeReason) {
        match reason {
            WakeReason::HandleInput(client_id, input) => {
                let updated = self.state.update_input(now, client_id, input);
                let entity_id = self.state.world().client(client_id).pawn_id().unwrap();
                match updated {
                    Ok(true) => self.update_entity_motion(now, entity_id),
                    Ok(false) => {},
                    Err(e) => warn!("update_input error: {}", e.description()),
                }
            },

            WakeReason::HandleAction(client_id, action) => {
                let updates = self.state.perform_action(now, client_id, action);
                for update in updates.into_iter() {
                    match update {
                        ChunkUpdate(cx, cy) => {
                            self.update_chunk(now, V2::new(cx, cy));
                        },
                    }
                }
            },

            WakeReason::PhysicsUpdate(entity_id) => {
                let updated = self.state.update_physics(now, entity_id, false);
                match updated {
                    Ok(true) => self.update_entity_motion(now, entity_id),
                    Ok(false) => {},
                    Err(e) => warn!("update_physics error: {}", e.description()),
                }
            },

            WakeReason::CheckView(client_id) => {
                self.update_viewport(now, client_id);
                self.wake_queue.push(now + 1000, WakeReason::CheckView(client_id));
            },
        }

        drop(self.state.world_mut().take_journal());
    }

    fn send(&self, client: &ObjectRef<world::Client>, resp: Response) {
        self.send_wire(client.wire_id(), resp);
    }

    fn send_wire(&self, wire_id: WireId, resp: Response) {
        self.resps.send((wire_id, resp)).unwrap();
    }

    fn reset_viewport(&mut self,
                      now: Time,
                      cid: ClientId,
                      first_init: bool) {
        let (old_region, new_region) = {
            let mut client = match self.state.world_mut().get_client_mut(cid) {
                Some(x) => x,
                None => return,
            };

            // Update the client's viewport state.  We don't use the normal update_viewport code
            // because we want to (mostly) ignore the old region.
            let old_region = client.view_state().region();
            let opt_pos = client.pawn().map(|p| p.pos(now));
            if let Some(pos) = opt_pos {
                // TODO: hardcoded constant based on entity size
                client.view_state_mut().update(pos + V3::new(16, 16, 0));
            }
            let new_region = client.view_state().region();

            (old_region, new_region)
        };

        if !first_init {
            for (x,y) in old_region.points() {
                self.state.unload_chunk(x, y);
            }
        }

        for (x,y) in new_region.points() {
            self.state.load_chunk(x, y);
        }


        let client = self.state.world().client(cid);
        // TODO: filter to only include entities within the viewport
        let entities = self.state.world().entities().collect::<Vec<_>>();

        // Send an Init message to reset the client.
        let info = msg::InitData {
            entity_id: client.pawn_id().unwrap_or(EntityId(-1)),
            camera_pos: (0, 0),
            chunks: 5 * 6,
            // TODO: probably want to make this field bigger than u8
            entities: entities.len() as u8,
        };
        self.send(&client, Response::Init(info));

        let camera_pos = client.camera_pos(now);
        let offset = chunk_offset(camera_pos.extend(0), client.chunk_offset());
        for (x,y) in new_region.points() {
            let idx = chunk_to_idx(x, y, offset);
            let data = self.state.get_terrain_rle16(x, y);
            self.send(&client, Response::TerrainChunk(idx as u16, data));
        }

        for entity in entities.iter() {
            let motion = entity_motion(now, entity.motion(), client.chunk_offset());
            let anim = entity.anim();
            self.send(&client, Response::EntityUpdate(entity.id(), motion, anim));
        }
    }

    fn update_viewport(&mut self,
                       now: Time,
                       cid: ClientId) {
        let (wire_id, result, offset) = {
            let mut client = match self.state.world_mut().get_client_mut(cid) {
                Some(x) => x,
                None => return,
            };
            let pos = client.pawn().unwrap().pos(now);
            // TODO: hardcoded constant based on entity size
            (client.wire_id(),
             client.view_state_mut().update(pos + V3::new(16, 16, 0)),
             chunk_offset(pos, client.chunk_offset()))
        };

        if let Some((old_region, new_region)) = result {
            for (x,y) in old_region.points().filter(|&(x,y)| !new_region.contains(x,y)) {
                use std::io::fs::File;
                let file = File::create(&Path::new(&*format!("{},{}.chunk", x, y))).unwrap();
                let mut sw = world::save::SaveWriter::new(file);
                sw.save_terrain_chunk(&self.state.world().terrain_chunk(V2::new(x, y))).unwrap();

                self.state.unload_chunk(x, y);
                let idx = chunk_to_idx(x, y, offset);
                self.send_wire(wire_id, Response::UnloadChunk(idx as u16));
            }

            for (x,y) in new_region.points().filter(|&(x,y)| !old_region.contains(x,y)) {
                self.state.load_chunk(x, y);
                let idx = chunk_to_idx(x, y, offset);
                let data = self.state.get_terrain_rle16(x, y);
                self.send_wire(wire_id, Response::TerrainChunk(idx as u16, data));
            }
        }
    }

    fn update_chunk(&mut self,
                    now: Time,
                    chunk_pos: V2) {
        // TODO: keep an index of which clients are viewing which chunks
        for c in self.state.world().clients() {
            let cx = chunk_pos.x;
            let cy = chunk_pos.y;
            if !c.view_state().region().contains(cx, cy) {
                continue;
            }

            let offset = chunk_offset(c.camera_pos(now).extend(0), c.chunk_offset());
            let idx = chunk_to_idx(cx, cy, offset);
            let data = self.state.get_terrain_rle16(cx, cy);
            self.send(&c, Response::TerrainChunk(idx as u16, data));
        }
    }

    fn update_entity_motion(&mut self,
                            now: Time,
                            eid: EntityId) {
        // TODO: send updates only to clients that might actually see them
        let entity = match self.state.world().get_entity(eid) {
            Some(e) => e,
            None => return,
        };
        let motion = entity.motion();

        for client in self.state.world().clients() {
            let wire_motion = entity_motion(now, motion, client.chunk_offset());
            self.send(&client, Response::EntityUpdate(eid, wire_motion, entity.anim()));
        }

        if motion.start_pos != motion.end_pos {
            self.wake_queue.push(motion.end_time(), WakeReason::PhysicsUpdate(eid));
        }
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
