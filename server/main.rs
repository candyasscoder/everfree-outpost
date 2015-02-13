#![crate_name = "backend"]
#![feature(unboxed_closures)]
#![feature(unsafe_destructor)]
#![allow(non_upper_case_globals)]
#![allow(unstable)]
#![allow(dead_code)]

#[macro_use] extern crate bitflags;
extern crate core;
extern crate libc;
#[macro_use] extern crate log;
extern crate serialize;
extern crate time;

extern crate collect;

extern crate physics;

use std::cmp;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::io;
use std::os;
use std::sync::mpsc::{Sender, Receiver, channel};
use std::thread::Thread;
use std::u8;
use serialize::json;

use physics::v3::{Vn, V3, V2};

use timer::WakeQueue;
use msg::Motion as WireMotion;
use msg::{Request, Response};
use input::{InputBits, ActionId};
use state::LOCAL_SIZE;
use state::StateChange::ChunkUpdate;
use data::Data;
use world::object::{ObjectRef, ObjectRefBase, ClientRef, InventoryRefMut};
use storage::Storage;
use view::ViewState;
use util::Cursor;
use util::{multimap_insert, multimap_remove};

use types::{Time, ToGlobal, ToLocal};
use types::{WireId, ClientId, EntityId, InventoryId};


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
mod storage;


fn read_json(path: &str) -> json::Json {
    use std::io::fs::File;
    let mut file = File::open(&Path::new(path)).unwrap();
    let json = json::from_reader(&mut file).unwrap();
    json
}

fn main() {
    let storage = Storage::new(Path::new(&os::args()[1]));

    let block_json = json::from_reader(&mut storage.open_block_data()).unwrap();
    let item_json = json::from_reader(&mut storage.open_item_data()).unwrap();
    let recipe_json = json::from_reader(&mut storage.open_recipe_data()).unwrap();
    let template_json = json::from_reader(&mut storage.open_template_data()).unwrap();
    let data = Data::from_json(block_json, item_json, recipe_json, template_json).unwrap();

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

    let state = state::State::new(&data, storage);
    let mut server = Server::new(resp_send, state);
    server.run(req_recv);
}


#[derive(Copy)]
pub enum WakeReason {
    HandleInput(ClientId, InputBits),
    HandleAction(ClientId, ActionId, u32),
    PhysicsUpdate(EntityId),
    CheckView(ClientId),
}

struct Server<'a> {
    resps: Sender<(WireId, Response)>,
    state: state::State<'a>,
    wake_queue: WakeQueue<WakeReason>,

    wire_id_map: HashMap<WireId, ClientId>,
    client_info: HashMap<ClientId, ClientInfo>,
    inventory_observers: HashMap<InventoryId, HashSet<WireId>>,
}

struct ClientInfo {
    wire_id: WireId,
    view_state: ViewState,
    observed_inventories: HashSet<InventoryId>,
}

impl<'a> Server<'a> {
    fn new(resps: Sender<(WireId, Response)>,
           state: state::State<'a>) -> Server<'a> {
        Server {
            resps: resps,
            state: state,
            wake_queue: WakeQueue::new(),

            wire_id_map: HashMap::new(),
            client_info: HashMap::new(),
            inventory_observers: HashMap::new(),
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
                    let client = self.state.load_client(&*name).unwrap();
                    (client.id(), client.pawn_id())
                };
                self.wire_id_map.insert(wire_id, cid);
                self.client_info.insert(cid, ClientInfo {
                    wire_id: wire_id,
                    view_state: ViewState::new(),
                    observed_inventories: HashSet::new(),
                });
                self.reset_viewport(now, cid);
                if let Some(eid) = eid {
                    self.update_entity_motion(now, eid);
                }
                self.wake_queue.push(now + 1000, WakeReason::CheckView(cid));
            },

            Request::Action(time, action, arg) => {
                let client_id = unwrap_or!(self.wire_to_client(wire_id));
                let time = cmp::max(time.to_global(now), now);
                self.wake_queue.push(time, WakeReason::HandleAction(client_id, ActionId(action), arg));
            },

            Request::UnsubscribeInventory(iid) => {
                if let Some(client_id) = self.wire_to_client(wire_id) {
                    self.client_info[client_id].observed_inventories.remove(&iid);
                    multimap_remove(&mut self.inventory_observers, iid, wire_id);
                }
            },

            Request::MoveItem(from_iid, to_iid, item_id, amount) => {
                let real_amount = {
                    // TODO: error handling
                    let i1 = self.state.world().get_inventory(from_iid);
                    let i2 = self.state.world().get_inventory(to_iid);
                    if let (Some(i1), Some(i2)) = (i1, i2) {
                        let count1 = i1.count(item_id);
                        let count2 = i2.count(item_id);
                        cmp::min(cmp::min(count1 as u16, (u8::MAX - count2) as u16), amount) as i16
                    } else {
                        0
                    }
                };
                if real_amount > 0 {
                    self.state.world_mut().inventory_mut(from_iid).update(item_id, -real_amount).unwrap();
                    self.state.world_mut().inventory_mut(to_iid).update(item_id, real_amount).unwrap();
                }
            },

            Request::CraftRecipe(station_id, inventory_id, recipe_id, count) => {
                // TODO: error handling
                let recipe = self.state.world().data().recipes.recipe(recipe_id);

                let real_count = {
                    let _ = station_id; // TODO
                    let i = self.state.world().inventory(inventory_id);
                    let mut count = count as u8;

                    for (&item_id, &num_required) in recipe.inputs.iter() {
                        count = cmp::min(count, i.count(item_id) / num_required);
                    }

                    for (&item_id, &num_produced) in recipe.outputs.iter() {
                        count = cmp::min(count, (u8::MAX - i.count(item_id)) / num_produced);
                    }

                    count as i16
                };

                if real_count > 0 {
                    let mut i = self.state.world_mut().inventory_mut(inventory_id);

                    for (&item_id, &num_required) in recipe.inputs.iter() {
                        i.update(item_id, -real_count * num_required as i16).unwrap();
                    }

                    for (&item_id, &num_produced) in recipe.outputs.iter() {
                        i.update(item_id, real_count * num_produced as i16).unwrap();
                    }
                }
            },

            Request::AddClient => {
            },

            Request::RemoveClient => {
                if let Some(client_id) = self.wire_to_client(wire_id) {
                    let region = self.client_info[client_id].view_state.region();
                    for p in region.points() {
                        self.state.unload_chunk(p.x, p.y);
                    }

                    self.state.unload_client(client_id).unwrap();
                    self.wire_id_map.remove(&wire_id);
                    let info = self.client_info.remove(&client_id).unwrap();
                    for &iid in info.observed_inventories.iter() {
                        multimap_remove(&mut self.inventory_observers, iid, wire_id);
                    }
                }
                self.resps.send((wire_id, Response::ClientRemoved)).unwrap();
            },

            Request::BadMessage(opcode) => {
                warn!("unrecognized opcode from connection {:?}: {:x}",
                      wire_id, opcode.unwrap());
            },
        }

        self.process_journal(now);
    }

    fn handle_wake(&mut self,
                   now: Time,
                   reason: WakeReason) {
        match reason {
            WakeReason::HandleInput(client_id, input) => {
                let result = self.state.update_input(now, client_id, input);
                if let Err(e) = result {
                    warn!("update_input: {}", e);
                }
            },

            WakeReason::HandleAction(client_id, action, arg) => {
                let result = self.state.perform_action(now, client_id, action, arg);
                if let Err(e) = result {
                    warn!("perform_action: {}", e);
                }
            },

            WakeReason::PhysicsUpdate(entity_id) => {
                let result = self.state.update_physics(now, entity_id, false);
                if let Err(e) = result {
                    warn!("update_physics: {}", e);
                }
            },

            WakeReason::CheckView(client_id) => {
                self.update_viewport(now, client_id);
                self.wake_queue.push(now + 1000, WakeReason::CheckView(client_id));
            },
        }

        self.process_journal(now);
    }

    fn process_journal(&mut self, now: Time) {
        state::State::process_journal(Cursor::new(self, |s| &mut s.state), |st, u| {
            let mut s = st.up();
            match u {
                // TODO: Client, Inventory lifecycle events
                world::Update::ClientPawnChange(cid) => s.reset_viewport(now, cid),
                world::Update::ChunkInvalidate(pos) => s.update_chunk(now, pos),
                world::Update::EntityMotionChange(eid) => s.update_entity_motion(now, eid),
                world::Update::InventoryUpdate(iid, item_id, old_count, new_count) => {
                    let s = &mut **s;
                    if let Some(wire_ids) = s.inventory_observers.get(&iid) {
                        for &wire_id in wire_ids.iter() {
                            let resp = Response::InventoryUpdate(
                                    iid, vec![(item_id, old_count, new_count)]);
                            s.resps.send((wire_id, resp)).unwrap();
                        }
                    }
                },

                world::Update::ClientShowInventory(cid, iid) => {
                    // TODO: check cid, iid are valid.
                    let wire_id = s.client_info[cid].wire_id;
                    s.send_wire(wire_id, Response::OpenDialog(0, vec![iid.unwrap()]));
                    s.subscribe_inventory(cid, iid);
                },

                world::Update::ClientOpenContainer(cid, iid1, iid2) => {
                    // TODO: check cid, iid1, iid2 are all valid.
                    let wire_id = s.client_info[cid].wire_id;
                    s.send_wire(wire_id, Response::OpenDialog(1, vec![iid1.unwrap(),
                                                                      iid2.unwrap()]));
                    s.subscribe_inventory(cid, iid1);
                    s.subscribe_inventory(cid, iid2);
                },

                world::Update::ClientOpenCrafting(cid, sid, iid) => {
                    // TODO: check ids are all valid
                    let station_type = s.state.world().structure(sid).template_id();

                    let wire_id = s.client_info[cid].wire_id;
                    s.send_wire(wire_id, Response::OpenCrafting(station_type,
                                                                sid,
                                                                iid));
                    s.subscribe_inventory(cid, iid);
                },

                _ => {},
            }
        });
    }

    fn subscribe_inventory(&mut self, cid: ClientId, iid: InventoryId) {
        self.client_info[cid].observed_inventories.insert(iid);
        let wire_id = self.client_info[cid].wire_id;
        multimap_insert(&mut self.inventory_observers, iid, wire_id);

        // TODO: might crash if inventory is destroyed in a later event
        let i = self.state.world().inventory(iid);
        let contents_map = i.contents();
        let mut contents = Vec::with_capacity(contents_map.len());
        for (&item_id, &count) in contents_map.iter() {
            contents.push((item_id, 0, count));
        }
        self.send_wire(wire_id, Response::InventoryUpdate(iid, contents));
    }

    fn send(&self, client: &ObjectRef<world::Client>, resp: Response) {
        let wire_id = self.client_info[client.id()].wire_id;
        self.send_wire(wire_id, resp);
    }

    fn send_wire(&self, wire_id: WireId, resp: Response) {
        self.resps.send((wire_id, resp)).unwrap();
    }

    fn reset_viewport(&mut self,
                      now: Time,
                      cid: ClientId) {
        let (old_region, new_region) = {
            let client = match self.state.world().get_client(cid) {
                Some(x) => x,
                None => return,
            };

            let info = match self.client_info.get_mut(&cid) {
                Some(x) => x,
                None => {
                    warn!("found client {:?} in world, but not in client_info", cid);
                    return;
                },
            };

            let old_region = info.view_state.region();

            match client.pawn().map(|p| p.pos(now)) {
                Some(pos) => { info.view_state.update(pos); },
                None => { info.view_state.clear(); },
            }
            let new_region = info.view_state.region();

            (old_region, new_region)
        };

        for p in old_region.points() {
            self.state.unload_chunk(p.x, p.y);
        }

        for p in new_region.points() {
            self.state.load_chunk(p.x, p.y);
        }


        let client = self.state.world().client(cid);
        // TODO: filter to only include entities within the viewport
        let entities = self.state.world().entities().collect::<Vec<_>>();

        // Send an Init message to reset the client.
        let info = msg::InitData {
            entity_id: client.pawn_id().unwrap_or(EntityId(-1)),
            // TODO: send pawn position, if available
            camera_pos: (0, 0),
            chunks: new_region.volume() as u8,
            // TODO: probably want to make this field bigger than u8
            entities: entities.len() as u8,
        };
        self.send(&client, Response::Init(info));

        let camera_pos = client.camera_pos(now);
        let offset = chunk_offset(camera_pos.extend(0), client.chunk_offset());
        for p in new_region.points() {
            let (x, y) = (p.x, p.y);
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
            let client = match self.state.world().get_client(cid) {
                Some(x) => x,
                None => return,
            };
            // TODO: bad error handling
            let pos = client.pawn().unwrap().pos(now);

            let info = &mut self.client_info[client.id()];
            (info.wire_id,
             // TODO: hardcoded constant based on entity size
             info.view_state.update(pos + V3::new(16, 16, 0)),
             chunk_offset(pos, client.chunk_offset()))
        };

        if let Some((old_region, new_region)) = result {
            // TODO: if the two regions don't overlap at all, send an "init" message so the player
            // gets a loading screen
            for p in old_region.points().filter(|&p| !new_region.contains(p)) {
                let (x, y) = (p.x, p.y);
                self.state.unload_chunk(x, y);
                let idx = chunk_to_idx(x, y, offset);
                self.send_wire(wire_id, Response::UnloadChunk(idx as u16));
            }

            for p in new_region.points().filter(|&p| !old_region.contains(p)) {
                let (x, y) = (p.x, p.y);
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
            if !self.client_info[c.id()].view_state.region().contains(chunk_pos) {
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
