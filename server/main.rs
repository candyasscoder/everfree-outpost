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
                    observed_inventories: HashSet::new(),
                });
                self.vision.add_client(cid, Region::empty(), &mut DummyCallbacks);
                self.reset_viewport(now, cid, |s| {
                    if let Some(eid) = eid {
                        s.state.update_physics(now, eid, true).unwrap();
                    }
                });
                self.wake_queue.push(now + 1000, WakeReason::CheckView(cid));

                self.server_msg(wire_id, &*format!("logged in as {}", name));
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

            Request::Chat(msg) => {
                if let Some(cid) = self.wire_to_client(wire_id) {
                    let msg_out = format!("<{}>\t{}",
                                          self.state.world().client(cid).name(),
                                          msg);
                    for &out_wire_id in self.wire_id_map.keys() {
                        self.send_wire(out_wire_id,
                                       Response::ChatUpdate(msg_out.clone()));
                    }
                }
            },

            Request::AddClient => {
            },

            Request::RemoveClient => {
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
            let s = &mut **s;
            match u {
                world::Update::TerrainChunkCreated(pos) =>
                    s.vision.add_chunk(pos, mk_callbacks!(s, now)),
                world::Update::TerrainChunkDestroyed(pos) =>
                    s.vision.remove_chunk(pos, mk_callbacks!(s, now)),
                world::Update::ChunkInvalidate(pos) =>
                    s.vision.update_chunk(pos, mk_callbacks!(s, now)),

                world::Update::EntityCreated(eid) => {
                    // TODO: error handling
                    let area = entity_area(s.state.world().entity(eid));
                    s.vision.add_entity(eid, area, mk_callbacks!(s, now))
                },
                world::Update::EntityDestroyed(eid) => {
                    s.vision.remove_entity(eid, mk_callbacks!(s, now));
                },
                world::Update::EntityMotionChange(eid) => {
                    // TODO: error handling
                    let entity = s.state.world().entity(eid);
                    let area = entity_area(entity);
                    s.vision.set_entity_area(eid, area, mk_callbacks!(s, now));

                    if entity.motion().start_pos != entity.motion().end_pos {
                        s.wake_queue.push(entity.motion().end_time(),
                                          WakeReason::PhysicsUpdate(eid));
                    }
                },

                // TODO: Client, Inventory lifecycle events
                world::Update::ClientPawnChange(cid) => s.reset_viewport(now, cid, |_| {}),
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

    // `callback` runs just after all chunks have been loaded.
    fn reset_viewport<CB>(&mut self,
                          now: Time,
                          cid: ClientId,
                          callback: CB)
            where CB: FnOnce(&mut Server) {
        let old_region = match self.vision.client_view_area(cid) {
            Some(x) => x,
            None => return,
        };

        for p in old_region.points() {
            self.state.unload_chunk(p.x, p.y);
        }

        self.vision.set_client_view(cid, Region::empty(), &mut DummyCallbacks);


        let new_region = {
            // TODO: warn on None? - may indicate inconsistency between World and Vision
            let client = unwrap_or!(self.state.world().get_client(cid));

            // TODO: make sure return is the right thing to do on None
            let pawn = unwrap_or!(client.pawn());

            view::vision_region(pawn.pos(now))
        };

        for p in new_region.points() {
            self.state.load_chunk(p.x, p.y);
        }

        callback(self);

        self.process_journal(now);

        let mut buffers = ClientInitBuffers::new(cid);
        self.vision.set_client_view(cid, new_region, &mut buffers);
        let buffers = buffers;


        let client = self.state.world().client(cid);

        info!("sending {} + {}", buffers.chunks.len(), buffers.entities.len());
        // Send an Init message to reset the client.
        let info = msg::InitData {
            entity_id: client.pawn_id()
                             .expect("pawn was checked to be Some above"),
            // TODO: send pawn position, if available
            camera_pos: (0, 0),
            chunks: buffers.chunks.len() as u8,
            // TODO: probably want to make this field bigger than u8
            entities: buffers.entities.len() as u8,
        };

        self.send(&client, Response::Init(info));

        // Now send the visible chunks and entities.
        let camera_pos = client.camera_pos(now);
        let offset = chunk_offset(camera_pos.extend(0), client.chunk_offset());

        for p in buffers.chunks.into_iter() {
            let idx = chunk_to_idx(p.x, p.y, offset);
            let data = self.state.get_terrain_rle16(p.x, p.y);
            self.send(&client, Response::TerrainChunk(idx as u16, data));
        }

        for eid in buffers.entities.into_iter() {
            let entity = self.state.world().get_entity(eid)
                             .expect("inconsistency between World and Vision");
            let motion = entity_motion(now, entity.motion(), client.chunk_offset());
            let anim = entity.anim();
            self.send(&client, Response::EntityUpdate(entity.id(), motion, anim));
        }
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


fn entity_area(e: ObjectRef<world::Entity>) -> util::SmallVec<V2> {
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
