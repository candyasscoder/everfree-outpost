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

use physics::v3::{Vn, V3, V2, scalar, Region};
use physics::{CHUNK_SIZE, TILE_SIZE};

use timer::WakeQueue;
use msg::Motion as WireMotion;
use msg::{Request, Response};
use input::{InputBits, ActionId};
use state::LOCAL_SIZE;
use state::StateChange::ChunkUpdate;
use data::Data;
use world::object::{ObjectRef, ObjectRefBase, ClientRef, InventoryRefMut};
use storage::Storage;
use view::{Vision, VisionCallbacks};
use util::Cursor;
use util::{multimap_insert, multimap_remove};
use util::StringResult;

use types::{Time, ToGlobal, ToLocal};
use types::{WireId, CONTROL_WIRE_ID};
use types::{ClientId, EntityId, InventoryId};


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

    let mut state = state::State::new(&data, storage);
    state.load_world();
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
    vision: Vision,

    wire_id_map: HashMap<WireId, ClientId>,
    client_info: HashMap<ClientId, ClientInfo>,
    inventory_observers: HashMap<InventoryId, HashSet<WireId>>,
}

struct ClientInfo {
    wire_id: WireId,
    observed_inventories: HashSet<InventoryId>,
}

macro_rules! mk_callbacks {
    ($self_:expr, $now:expr) => {
        &mut MessageCallbacks {
            state: &$self_.state,
            wire: &$self_.resps,
            client_info: &$self_.client_info,
            now: $now,
        }
    };
}

impl<'a> Server<'a> {
    fn new(resps: Sender<(WireId, Response)>,
           state: state::State<'a>) -> Server<'a> {
        Server {
            resps: resps,
            state: state,
            wake_queue: WakeQueue::new(),
            vision: Vision::new(),

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
        if wire_id == CONTROL_WIRE_ID {
            self.handle_control_req(now, req);
        } else {
            if let Some(client_id) = self.wire_to_client(wire_id) {
                let result = self.handle_client_req(now, wire_id, client_id, req);
                if let Err(e) = result {
                    warn!("error handling request for {:?} ({:?}): {}",
                          client_id, wire_id, e);
                    self.kick_wire(now, wire_id, "bad request");
                }
            } else {
                let result = self.handle_pre_login_client_req(now, wire_id, req);
                if let Err(e) = result {
                    warn!("error handling request for {:?}: {}",
                          wire_id, e);
                    self.kick_wire(now, wire_id, "bad request");
                }
            }
        }

        self.process_journal(now);
    }

    fn handle_control_req(&mut self,
                          _now: Time,
                          req: Request) {
        match req {
            Request::AddClient(_wire_id) => {
            },

            Request::RemoveClient(wire_id) => {
                if let Some(client_id) = self.wire_to_client(wire_id) {
                    // TODO: error handling
                    let region = self.vision.client_view_area(client_id).unwrap();
                    for p in region.points() {
                        self.state.unload_chunk(p.x, p.y);
                    }

                    self.state.unload_client(client_id).unwrap();
                    self.wire_id_map.remove(&wire_id);
                    let info = self.client_info.remove(&client_id).unwrap();
                    for &iid in info.observed_inventories.iter() {
                        multimap_remove(&mut self.inventory_observers, iid, wire_id);
                    }
                    self.vision.remove_client(client_id, &mut DummyCallbacks);
                }
                self.resps.send((CONTROL_WIRE_ID, Response::ClientRemoved(wire_id))).unwrap();
            },

            _ => warn!("bad control request: {:?}", req),
        }
    }

    fn handle_pre_login_client_req(&mut self,
                                   now: Time,
                                   wire_id: WireId,
                                   req: Request) -> StringResult<()> {
        match req {
            Request::Ping(cookie) => {
                self.send_wire(wire_id, Response::Pong(cookie, now.to_local()));
            },

            Request::Login(_secret, name) => {
                log!(10, "login request for {}", name);

                let (cid, eid) = {
                    let client = try!(self.state.load_client(&*name));
                    (client.id(), client.pawn_id())
                };
                self.wire_id_map.insert(wire_id, cid);
                self.client_info.insert(cid, ClientInfo {
                    wire_id: wire_id,
                    observed_inventories: HashSet::new(),
                });

                let view_region = if let Some(eid) = eid {
                    // OK: obtained eid from client.pawn_id(), which is always valid (or None)
                    let pawn = self.state.world().entity(eid);
                    view::vision_region(pawn.pos(now))
                } else {
                    Region::empty()
                };

                for p in view_region.points() {
                    self.state.load_chunk(p.x, p.y);
                }

                if let Some(eid) = eid {
                    // OK: obtained eid from client.pawn_id(), which is always valid (or None)
                    self.state.update_physics(now, eid, true).unwrap();
                }

                self.vision.add_client(cid, view_region, mk_callbacks!(self, now));

                self.wake_queue.push(now + 1000, WakeReason::CheckView(cid));
                self.server_msg(wire_id, &*format!("logged in as {}", name));
            },

            _ => {
                fail!("bad request (pre-login): {:?}", req);
            },
        }

        Ok(())
    }

    fn handle_client_req(&mut self,
                         now: Time,
                         wire_id: WireId,
                         cid: ClientId,
                         req: Request) -> StringResult<()> {
        match req {
            Request::Ping(cookie) => {
                self.send_wire(wire_id, Response::Pong(cookie, now.to_local()));
            },

            Request::Input(time, input) => {
                let time = cmp::max(time.to_global(now), now);
                let input = InputBits::from_bits_truncate(input);
                self.wake_queue.push(time, WakeReason::HandleInput(cid, input));
            },

            Request::Action(time, action, arg) => {
                let time = cmp::max(time.to_global(now), now);
                self.wake_queue.push(time, WakeReason::HandleAction(cid, ActionId(action), arg));
            },

            Request::UnsubscribeInventory(iid) => {
                self.client_info[cid].observed_inventories.remove(&iid);
                multimap_remove(&mut self.inventory_observers, iid, wire_id);
            },

            Request::MoveItem(from_iid, to_iid, item_id, amount) => {
                let real_amount = {
                    let i1 = unwrap!(self.state.world().get_inventory(from_iid));
                    let i2 = unwrap!(self.state.world().get_inventory(to_iid));
                    let count1 = i1.count(item_id);
                    let count2 = i2.count(item_id);
                    cmp::min(cmp::min(count1 as u16, (u8::MAX - count2) as u16), amount) as i16
                };
                if real_amount > 0 {
                    // OK: inventory IDs have already been checked.
                    self.state.world_mut().inventory_mut(from_iid).update(item_id, -real_amount);
                    self.state.world_mut().inventory_mut(to_iid).update(item_id, real_amount);
                }
            },

            Request::CraftRecipe(station_id, inventory_id, recipe_id, count) => {
                // TODO: error handling
                let recipe = unwrap!(self.state.world().data().recipes.get_recipe(recipe_id));

                let _ = station_id; // TODO
                let mut i = unwrap!(self.state.world_mut().get_inventory_mut(inventory_id));

                let real_count = {
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
                    for (&item_id, &num_required) in recipe.inputs.iter() {
                        i.update(item_id, -real_count * num_required as i16);
                    }

                    for (&item_id, &num_produced) in recipe.outputs.iter() {
                        i.update(item_id, real_count * num_produced as i16);
                    }
                }
            },

            Request::Chat(msg) => {
                let msg_out = format!("<{}>\t{}",
                                      self.state.world().client(cid).name(),
                                      msg);
                for &out_wire_id in self.wire_id_map.keys() {
                    self.send_wire(out_wire_id,
                                   Response::ChatUpdate(msg_out.clone()));
                }
            },

            _ => {
                fail!("bad request: {:?}", req);
            },
        }

        Ok(())
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
        let mut invalidated_chunks = HashSet::new();

        state::State::process_journal(Cursor::new(self, |s| &mut s.state), |st, u| {
            let mut s = st.up();
            let s = &mut **s;
            match u {
                world::Update::TerrainChunkCreated(pos) => {
                    s.vision.add_chunk(pos, &mut DummyCallbacks);
                    invalidated_chunks.insert(pos);
                },
                world::Update::TerrainChunkDestroyed(pos) =>
                    s.vision.remove_chunk(pos, mk_callbacks!(s, now)),
                world::Update::ChunkInvalidate(pos) => {
                    // Deduplicate ChunkInvalidate updates due to spam on chunk load.
                    invalidated_chunks.insert(pos);
                },

                world::Update::EntityCreated(eid) => {
                    // TODO: error handling
                    let area = entity_area(&s.state.world().entity(eid));
                    s.vision.add_entity(eid, area, mk_callbacks!(s, now))
                },
                world::Update::EntityDestroyed(eid) => {
                    s.vision.remove_entity(eid, mk_callbacks!(s, now));
                },
                world::Update::EntityMotionChange(eid) => {
                    // TODO: error handling
                    let entity = s.state.world().entity(eid);
                    let area = entity_area(&entity);
                    s.vision.set_entity_area(eid, area, mk_callbacks!(s, now));

                    if entity.motion().start_pos != entity.motion().end_pos {
                        s.wake_queue.push(entity.motion().end_time(),
                                          WakeReason::PhysicsUpdate(eid));
                    }
                },

                // TODO: Client, Inventory lifecycle events
                world::Update::ClientCreated(cid) |
                world::Update::ClientPawnChange(cid) => {
                    {
                        let client = s.state.world().client(cid);

                        let info = msg::InitData {
                            entity_id: client.pawn_id().unwrap_or(EntityId(-1)),
                            camera_pos: (0, 0),
                            chunks: 0,
                            entities: 0,
                        };
                        s.send(&client, Response::Init(info));
                    }

                    s.update_viewport(now, cid);
                },
                world::Update::InventoryUpdate(iid, item_id, old_count, new_count) => {
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

        for &pos in invalidated_chunks.iter() {
            self.vision.update_chunk(pos, mk_callbacks!(self, now));
        }
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

    fn client_info(&self, client: &ObjectRef<world::Client>) -> &ClientInfo {
        self.client_info.get(&client.id())
            .expect("inconsistency between world and client_info")
    }

    fn send(&self, client: &ObjectRef<world::Client>, resp: Response) {
        let wire_id = self.client_info(client).wire_id;
        self.send_wire(wire_id, resp);
    }

    fn send_wire(&self, wire_id: WireId, resp: Response) {
        self.resps.send((wire_id, resp)).unwrap();
    }

    fn kick_wire(&mut self, now: Time, wire_id: WireId, msg: &str) {
        self.resps.send((wire_id, Response::KickReason(String::from_str(msg)))).unwrap();
        self.handle_control_req(now, Request::RemoveClient(wire_id));
    }

    fn update_viewport(&mut self,
                       now: Time,
                       cid: ClientId) {
        let old_region = match self.vision.client_view_area(cid) {
            Some(x) => x,
            None => return,
        };

        let new_region = {
            // TODO: warn on None? - may indicate inconsistency between World and Vision
            let client = unwrap_or!(self.state.world().get_client(cid));

            // TODO: make sure return is the right thing to do on None
            let pawn = unwrap_or!(client.pawn());

            view::vision_region(pawn.pos(now))
        };

        for p in old_region.points().filter(|&p| !new_region.contains(p)) {
            self.state.unload_chunk(p.x, p.y);
        }

        for p in new_region.points().filter(|&p| !old_region.contains(p)) {
            self.state.load_chunk(p.x, p.y);
        }

        self.vision.set_client_view(cid, new_region, mk_callbacks!(self, now));
    }

    fn server_msg(&self, wire_id: WireId, msg: &str) {
        self.send_wire(wire_id, Response::ChatUpdate(format!("***\t{}", msg)));
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


// TODO: remove the & once copying ObjectRefs doesn't cause memory corruption (rustc dcaeb6aa2
// 2015-01-18 has this bug)
// NB: I used this same workaround in a bunch of other places and I don't remember where
fn entity_area(e: &ObjectRef<world::Entity>) -> util::SmallVec<V2> {
    let motion = e.motion();
    let mut result = util::SmallVec::new();

    let scale = scalar(CHUNK_SIZE * TILE_SIZE);
    let start_chunk = motion.start_pos.reduce().div_floor(scale);
    let end_chunk = motion.end_pos.reduce().div_floor(scale);

    result.push(start_chunk);
    if end_chunk != start_chunk {
        result.push(end_chunk);
    }
    result
}

fn entity_appearance(_: &world::Entity) -> u32 {
    0
}


struct DummyCallbacks;

impl VisionCallbacks for DummyCallbacks {
}

struct MessageCallbacks<'a, 'd: 'a> {
    state: &'a state::State<'d>,
    wire: &'a Sender<(WireId, Response)>,
    client_info: &'a HashMap<ClientId, ClientInfo>,
    now: Time,
}

impl<'a, 'd> VisionCallbacks for MessageCallbacks<'a, 'd> {
    fn chunk_update(&mut self, cid: ClientId, pos: V2) {
        let c = unwrap_or!(self.state.world().get_client(cid));

        let wire_id = self.client_info[cid].wire_id;

        let offset = chunk_offset(c.camera_pos(self.now).extend(0), c.chunk_offset());
        let idx = chunk_to_idx(pos.x, pos.y, offset);
        let data = self.state.get_terrain_rle16(pos.x, pos.y);
        self.wire.send((wire_id, Response::TerrainChunk(idx as u16, data))).unwrap();
    }

    fn entity_appear(&mut self, cid: ClientId, eid: EntityId) {
        let info = unwrap_or!(self.client_info.get(&cid));
        let entity = unwrap_or!(self.state.world().get_entity(eid));

        let appearance = entity_appearance(&*entity);

        self.wire.send((info.wire_id, Response::EntityAppear(eid, appearance))).unwrap();
    }

    fn entity_disappear(&mut self, cid: ClientId, eid: EntityId) {
        let info = unwrap_or!(self.client_info.get(&cid));
        let client = unwrap_or!(self.state.world().get_client(cid));
        let entity = unwrap_or!(self.state.world().get_entity(eid));

        let motion = entity.motion();
        // TODO: we don't actually need the whole wire_motion here, just the local start_time
        let wire_motion = entity_motion(self.now, motion, client.chunk_offset());
        self.wire.send((info.wire_id, Response::EntityGone(eid, wire_motion.start_time))).unwrap();
    }

    fn entity_update(&mut self, cid: ClientId, eid: EntityId) {
        let info = unwrap_or!(self.client_info.get(&cid));
        let client = unwrap_or!(self.state.world().get_client(cid));
        let entity = unwrap_or!(self.state.world().get_entity(eid));

        let motion = entity.motion();
        let wire_motion = entity_motion(self.now, motion, client.chunk_offset());
        self.wire.send((info.wire_id, Response::EntityUpdate(eid, wire_motion, entity.anim())))
            .unwrap();
    }
}

struct ClientInitBuffers {
    expect_cid: ClientId,
    chunks: Vec<V2>,
    entities: Vec<EntityId>,
}

impl VisionCallbacks for ClientInitBuffers {
    fn chunk_update(&mut self, cid: ClientId, pos: V2) {
        assert!(cid == self.expect_cid);
        self.chunks.push(pos);
    }

    fn chunk_disappear(&mut self, cid: ClientId, pos: V2) {
        panic!("chunks shouldn't diasppear during client init (cid={:?}, pos={:?})", cid, pos);
    }

    fn entity_update(&mut self, cid: ClientId, eid: EntityId) {
        assert!(cid == self.expect_cid);
        self.entities.push(eid);
    }

    fn entity_disappear(&mut self, cid: ClientId, eid: EntityId) {
        panic!("entities shouldn't diasppear during client init (cid={:?}, eid={:?})", cid, eid);
    }
}

impl ClientInitBuffers {
    fn new(cid: ClientId) -> ClientInitBuffers {
        ClientInitBuffers {
            expect_cid: cid,
            chunks: Vec::new(),
            entities: Vec::new(),
        }
    }
}
