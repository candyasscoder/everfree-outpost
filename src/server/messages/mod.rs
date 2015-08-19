use std::cmp;
use std::error::Error;
use std::mem;
use std::sync::mpsc::{Sender, Receiver};

use types::*;
use util::StringResult;
use util::now;
use libphysics::TILE_SIZE;

use auth::Secret;
use input::InputBits;
use msg::{Request, Response, InitData, ExtraArg};
use world::Motion;

use self::clients::Clients;


mod clients;


pub struct Messages {
    send: Sender<(WireId, Response)>,
    recv: Receiver<(WireId, Request)>,
    clients: Clients,
    time_base: Time,
}

pub enum Event {
    Control(ControlEvent),
    Wire(WireId, WireEvent),
    Client(ClientId, ClientEvent),
}


pub enum ControlEvent {
    OpenWire(WireId),
    CloseWire(WireId, Option<ClientId>),
    ReplCommand(u16, String),
    Shutdown,
    Restart(bool, bool),
}

pub enum WireEvent {
    Login(String, Secret),
    Register(String, Secret, u32),
    BadRequest,
}

pub enum ClientEvent {
    Input(Time, InputBits),
    UnsubscribeInventory(InventoryId),
    MoveItem(InventoryId, InventoryId, ItemId, u16),
    CraftRecipe(StructureId, InventoryId, RecipeId, u16),
    Chat(String),

    Interact(Time, Option<ExtraArg>),
    UseItem(Time, ItemId, Option<ExtraArg>),
    UseAbility(Time, ItemId, Option<ExtraArg>),

    BadRequest,
}


#[derive(Debug, Clone)]
pub enum ControlResponse {
    WireClosed(WireId),
    ReplResult(u16, String),
}

#[derive(Debug, Clone)]
pub enum WireResponse {
    RegisterResult(u32, String),
    KickReason(String),
}

#[derive(Debug, Clone)]
pub enum SyncKind {
    Loading,
    Ok,
    Reset,
    Refresh,
}

#[derive(Debug, Clone)]
pub enum ClientResponse {
    Init(Option<EntityId>, Time, u32, u32),

    TerrainChunk(V2, Vec<u16>),
    UnloadChunk(V2),

    EntityAppear(EntityId, u32, String),
    EntityUpdate(EntityId, Motion, AnimId),
    EntityGone(EntityId, Time),

    StructureAppear(StructureId, TemplateId, V3),
    StructureGone(StructureId),

    //InventoryAppear(InventoryId, Vec<(ItemId, u8)>),
    InventoryUpdate(InventoryId, Vec<(ItemId, u8, u8)>),
    //InventoryGone(InventoryId),
    
    PlaneFlags(u32),
    SyncStatus(SyncKind),

    GetInteractArgs(u32, ExtraArg),
    GetUseItemArgs(ItemId, u32, ExtraArg),
    GetUseAbilityArgs(ItemId, u32, ExtraArg),

    OpenDialog(Dialog),
    MainInventory(InventoryId),
    AbilityInventory(InventoryId),
    ChatUpdate(String),
    KickReason(String),
}

#[derive(Debug, Clone)]
pub enum Dialog {
    Inventory(InventoryId),
    Container(InventoryId, InventoryId),
    Crafting(TemplateId, StructureId, InventoryId),
}


/// Opaque wrapper around the low-level event type.
pub struct MessageEvent((WireId, Request));

fn cast_receiver(recv: &Receiver<(WireId, Request)>) -> &Receiver<MessageEvent> {
    unsafe { mem::transmute(recv) }
}


impl Messages {
    pub fn new(recv: Receiver<(WireId, Request)>,
               send: Sender<(WireId, Response)>) -> Messages {
        Messages {
            send: send,
            recv: recv,
            clients: Clients::new(),
            time_base: 0,
        }
    }


    // Time adjustment

    // Regarding timestamps: All Time values within this module, as well as all Times passed
    // to/from the Engine or transmitted to/from clients, are "world times" (that is, adjusted
    // using `time_base`).

    fn world_time(&self, unix_time: Time) -> Time {
        unix_time - self.time_base
    }

    fn world_now(&self) -> Time {
        self.world_time(now())
    }

    // NB: This is designed to be called only once, near the beginning of server startup.  Calling
    // it while the server is running may have strange effects.
    pub fn set_world_time(&mut self, unix_time: Time, world_time: Time) {
        self.time_base = unix_time - world_time;
        debug!("new time_base: {:x} (world_time {:x})", self.time_base, world_time);
    }

    fn from_world_time(&self, world_time: Time) -> Time {
        world_time + self.time_base
    }


    // Client lifecycle

    pub fn add_client(&mut self, cid: ClientId, wire_id: WireId, name: &str) {
        self.clients.add(cid, wire_id, name);
    }

    pub fn remove_client(&mut self, cid: ClientId) {
        self.clients.remove(cid);
    }

    pub fn wire_to_client(&self, wire_id: WireId) -> Option<ClientId> {
        self.clients.wire_to_client(wire_id)
    }

    pub fn client_to_wire(&self, cid: ClientId) -> Option<WireId> {
        self.clients.get(cid).map(|c| c.wire_id())
    }

    pub fn name_to_client(&self, name: &str) -> Option<ClientId> {
        self.clients.name_to_client(name)
    }

    pub fn clients_len(&self) -> usize {
        self.clients.len()
    }


    // Event processing

    pub fn receiver(&self) -> &Receiver<MessageEvent> {
        cast_receiver(&self.recv)
    }

    pub fn process(&mut self, evt: MessageEvent) -> Option<(Event, Time)> {
        let (wire_id, req) = evt.0;
        let now = self.world_now();
        self.handle_req(now, wire_id, req)
            .map(|evt| (evt, now))
    }

    fn handle_req(&mut self, now: Time, wire_id: WireId, req: Request) -> Option<Event> {
        if wire_id == CONTROL_WIRE_ID {
            self.handle_control_req(now, req)
        } else {
            if let Some(cid) = self.clients.wire_to_client(wire_id) {
                self.handle_client_req(now, wire_id, cid, req)
            } else {
                self.handle_pre_login_req(now, wire_id, req)
            }
        }
    }

    fn handle_control_req(&mut self, _now: Time, req: Request) -> Option<Event> {
        match req {
            Request::AddClient(wire_id) =>
                // Let the caller decide when to actually add the client.
                Some(Event::Control(ControlEvent::OpenWire(wire_id))),
            Request::RemoveClient(wire_id) => {
                // Let the caller decide when to actually remove the client.
                let opt_cid = self.clients.wire_to_client(wire_id);
                Some(Event::Control(ControlEvent::CloseWire(wire_id, opt_cid)))
            },
            Request::ReplCommand(cookie, cmd) =>
                Some(Event::Control(ControlEvent::ReplCommand(cookie, cmd))),
            Request::Shutdown =>
                Some(Event::Control(ControlEvent::Shutdown)),
            Request::Restart(server, client) =>
                Some(Event::Control(ControlEvent::Restart(server, client))),

            _ => {
                warn!("bad control request: {:?}", req);
                None
            },
        }
    }

    fn handle_pre_login_req(&mut self, now: Time, wire_id: WireId, req: Request) -> Option<Event> {
        match req {
            Request::Ping(cookie) => {
                self.send_raw(wire_id, Response::Pong(cookie, now.to_local()));
                None
            },
            Request::Login(name, secret) =>
                Some(Event::Wire(wire_id, WireEvent::Login(name, secret))),
            Request::Register(name, secret, appearance) =>
                Some(Event::Wire(wire_id, WireEvent::Register(name, secret, appearance))),
            _ => {
                warn!("bad pre-login request from {:?}: {:?}", wire_id, req);
                Some(Event::Wire(wire_id, WireEvent::BadRequest))
            },
        }
    }

    fn handle_client_req(&mut self,
                         now: Time,
                         wire_id: WireId,
                         cid: ClientId,
                         req: Request) -> Option<Event> {
        match self.try_handle_client_req(now, wire_id, req) {
            Ok(evt) => evt.map(|e| Event::Client(cid, e)),
            Err(e) => {
                warn!("bad request from {:?}: {}", cid, e.description());
                Some(Event::Client(cid, ClientEvent::BadRequest))
            },
        }
    }

    fn try_handle_client_req(&mut self,
                             now: Time,
                             wire_id: WireId,
                             req: Request) -> StringResult<Option<ClientEvent>> {
        match req {
            Request::Ping(cookie) => {
                self.send_raw(wire_id, Response::Pong(cookie, now.to_local()));
                Ok(None)
            },

            Request::Input(time, input) => {
                let time = cmp::max(time.to_global(now), now);
                let input = unwrap!(InputBits::from_bits(input));
                Ok(Some(ClientEvent::Input(time, input)))
            },

            Request::UnsubscribeInventory(iid) =>
                Ok(Some(ClientEvent::UnsubscribeInventory(iid))),

            Request::MoveItem(from_iid, to_iid, item_id, count) =>
                Ok(Some(ClientEvent::MoveItem(from_iid, to_iid, item_id, count))),

            Request::CraftRecipe(sid, iid, recipe_id, count) =>
                Ok(Some(ClientEvent::CraftRecipe(sid, iid, recipe_id, count))),

            Request::Chat(msg) =>
                Ok(Some(ClientEvent::Chat(msg))),


            Request::Interact(time) => {
                let time = cmp::max(time.to_global(now), now);
                Ok(Some(ClientEvent::Interact(time, None)))
            },

            Request::UseItem(time, item_id) => {
                let time = cmp::max(time.to_global(now), now);
                Ok(Some(ClientEvent::UseItem(time, item_id, None)))
            },

            Request::UseAbility(time, item_id) => {
                let time = cmp::max(time.to_global(now), now);
                Ok(Some(ClientEvent::UseAbility(time, item_id, None)))
            },


            Request::InteractWithArgs(time, args) => {
                let time = cmp::max(time.to_global(now), now);
                Ok(Some(ClientEvent::Interact(time, Some(args))))
            },

            Request::UseItemWithArgs(time, item_id, args) => {
                let time = cmp::max(time.to_global(now), now);
                Ok(Some(ClientEvent::UseItem(time, item_id, Some(args))))
            },

            Request::UseAbilityWithArgs(time, item_id, args) => {
                let time = cmp::max(time.to_global(now), now);
                Ok(Some(ClientEvent::UseAbility(time, item_id, Some(args))))
            },


            _ => fail!("bad request: {:?}", req),
        }
    }


    // Response sending

    fn send_raw(&self, wire_id: WireId, msg: Response) {
        self.send.send((wire_id, msg)).unwrap();
    }

    pub fn send_control(&self, resp: ControlResponse) {
        match resp {
            ControlResponse::WireClosed(wire_id) =>
                self.send_raw(CONTROL_WIRE_ID, Response::ClientRemoved(wire_id)),
            ControlResponse::ReplResult(cookie, msg) =>
                self.send_raw(CONTROL_WIRE_ID, Response::ReplResult(cookie, msg)),
        }
    }

    pub fn send_wire(&self, wire_id: WireId, resp: WireResponse) {
        match resp {
            WireResponse::RegisterResult(code, msg) =>
                self.send_raw(wire_id, Response::RegisterResult(code, msg)),
            WireResponse::KickReason(msg) =>
                self.send_raw(wire_id, Response::KickReason(msg)),
        }
    }

    pub fn send_client(&self, cid: ClientId, resp: ClientResponse) {
        let client = match self.clients.get(cid) {
            Some(x) => x,
            None => {
                debug!("can't send to client {:?} (no wire): {:?}", cid, resp);
                return;
            },
        };
        let wire_id = client.wire_id();

        match resp {
            ClientResponse::Init(opt_eid, time, cycle_base, cycle_ms) => {
                let data = InitData {
                    entity_id: opt_eid.unwrap_or(EntityId(-1_i32 as u32)),
                    now: time.to_local(),
                    cycle_base: cycle_base,
                    cycle_ms: cycle_ms,
                };
                self.send_raw(wire_id, Response::Init(data));
            },

            ClientResponse::TerrainChunk(cpos, data) => {
                let index = client.local_chunk_index(cpos);
                self.send_raw(wire_id, Response::TerrainChunk(index, data));
            },

            ClientResponse::UnloadChunk(cpos) => {
                let index = client.local_chunk_index(cpos);
                self.send_raw(wire_id, Response::UnloadChunk(index));
            },


            ClientResponse::EntityAppear(eid, appear, name) =>
                self.send_raw(wire_id, Response::EntityAppear(eid, appear, name)),

            ClientResponse::EntityUpdate(eid, motion, anim) => {
                let wire_motion = client.local_motion(motion);
                self.send_raw(wire_id, Response::EntityUpdate(eid, wire_motion, anim));
            },

            ClientResponse::EntityGone(eid, time) => {
                let time = time.to_local();
                self.send_raw(wire_id, Response::EntityGone(eid, time));
            },


            ClientResponse::StructureAppear(sid, template_id, pos) => {
                let local_pos = client.local_pos_tuple(pos * scalar(TILE_SIZE));
                self.send_raw(wire_id, Response::StructureAppear(sid, template_id, local_pos));
            },

            ClientResponse::StructureGone(sid) => {
                self.send_raw(wire_id, Response::StructureGone(sid));
            },


            ClientResponse::InventoryUpdate(iid, update) =>
                self.send_raw(wire_id, Response::InventoryUpdate(iid, update)),


            ClientResponse::PlaneFlags(flags) =>
                self.send_raw(wire_id, Response::PlaneFlags(flags)),

            ClientResponse::SyncStatus(kind) => {
                let arg = match kind {
                    SyncKind::Loading => 0,
                    SyncKind::Ok => 1,
                    SyncKind::Reset => 2,
                    SyncKind::Refresh => 3,
                };
                self.send_raw(wire_id, Response::SyncStatus(arg))
            },


            ClientResponse::GetInteractArgs(dialog_id, parts) =>
                self.send_raw(wire_id, Response::GetInteractArgs(dialog_id, parts)),

            ClientResponse::GetUseItemArgs(item_id, dialog_id, parts) =>
                self.send_raw(wire_id, Response::GetUseItemArgs(item_id, dialog_id, parts)),

            ClientResponse::GetUseAbilityArgs(item_id, dialog_id, parts) =>
                self.send_raw(wire_id, Response::GetUseAbilityArgs(item_id, dialog_id, parts)),


            ClientResponse::OpenDialog(dialog) => {
                match dialog {
                    Dialog::Inventory(iid) => 
                        self.send_raw(wire_id, Response::OpenDialog(0, vec![iid.unwrap()])),
                    Dialog::Container(iid1, iid2) => 
                        self.send_raw(wire_id, Response::OpenDialog(1, vec![iid1.unwrap(),
                                                                            iid2.unwrap()])),
                    Dialog::Crafting(template_id, sid, iid) =>
                        self.send_raw(wire_id, Response::OpenCrafting(template_id, sid, iid)),
                }
            },

            ClientResponse::MainInventory(iid) =>
                self.send_raw(wire_id, Response::MainInventory(iid)),

            ClientResponse::AbilityInventory(iid) =>
                self.send_raw(wire_id, Response::AbilityInventory(iid)),

            ClientResponse::ChatUpdate(msg) =>
                self.send_raw(wire_id, Response::ChatUpdate(msg)),

            ClientResponse::KickReason(msg) =>
                self.send_raw(wire_id, Response::KickReason(msg)),
        }
    }

    pub fn broadcast_clients(&self, resp: ClientResponse) {
        for (&cid, _) in self.clients.iter() {
            self.send_client(cid, resp.clone());
        }
    }
}
