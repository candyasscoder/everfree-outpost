use std::cmp;
use std::error::Error;
use std::sync::mpsc::{Sender, Receiver, Select};
use time;

use types::*;
use util::StringResult;

use auth::Secret;
use input::InputBits;
use input::Action;
use msg::{Request, Response};
use timer::WakeQueue;
use world::Motion;

pub struct Events {
    send: Sender<(WireId, Response)>,
    recv: Receiver<(WireId, Request)>,
    wake: WakeQueue<WakeReason>,
}

pub enum Event {
    Control(ControlEvent),
    Wire(WireId, WireEvent),
    Client(ClientId, ClientEvent),
    Other(OtherEvent),
}

#[derive(Copy)]
pub enum WakeReason {
    HandleInput(ClientId, InputBits),
    HandleAction(ClientId, Action),
    PhysicsUpdate(EntityId),
    CheckView(ClientId),
}


pub enum ControlEvent {
    AddClient(WireId),
    RemoveClient(WireId, Option<ClientId>),
    ReplCommand(u16, String),
}

pub enum WireEvent {
    Login(String, Secret),
    Register(String, Secret, u32),
    BadRequest,
}

pub enum ClientEvent {
    Input(InputBits),
    Action(Action),
    UnsubscribeInventory(InventoryId),
    MoveItem(InventoryId, InventoryId, ItemId, u16),
    CraftRecipe(StructureId, InventoryId, RecipeId, u16),
    Chat(String),
    CheckView,
    BadRequest,
}

pub enum OtherEvent {
    PhysicsUpdate(EntityId),
}


#[derive(Show)]
pub enum ControlResponse {
    ReplResult(u16, String),
}

#[derive(Show)]
pub enum WireResponse {
    RegisterResult(u32, String),
}

#[derive(Show)]
pub enum ClientResponse {
    TerrainChunk(V2, Vec<u16>),
    UnloadChunk(V2),

    EntityAppear(EntityId, u32, String),
    EntityUpdate(EntityId, Motion, AnimId),
    EntityGone(EntityId, Time),

    //InventoryAppear(InventoryId, Vec<(ItemId, u8)>),
    InventoryUpdate(InventoryId, Vec<(ItemId, u8, u8)>),
    //InventoryGone(InventoryId),

    OpenDialog(Dialog),
    ChatUpdate(String),
    KickReason(String),
}

#[derive(Show)]
pub enum Dialog {
    Inventory(InventoryId),
    Container(InventoryId, InventoryId),
    Crafting(TemplateId, StructureId, InventoryId),
}


impl Events {
    pub fn new(recv: Receiver<(WireId, Request)>,
               send: Sender<(WireId, Response)>) -> Events {
        Events {
            send: send,
            recv: recv,
            wake: WakeQueue::new(),
        }
    }

    pub fn next(&mut self) -> (Event, Time) {
        loop {
            enum Msg {
                Wake(()),
                Req((WireId, Request)),
            }


            let msg = {
                let wake_recv = self.wake.wait_recv(now());
                // select! can't handle 'self.recv' as a channel name.  Sigh...
                let select = Select::new();

                let mut wake_handle = select.handle(&wake_recv);
                let mut req_handle = select.handle(&self.recv);


                unsafe {
                    wake_handle.add();
                    req_handle.add();
                }

                let ready_id = select.wait();

                unsafe {
                    wake_handle.remove();
                    req_handle.remove();
                }


                if ready_id == wake_handle.id() {
                    Msg::Wake(wake_handle.recv().unwrap())
                } else {
                    Msg::Req(req_handle.recv().unwrap())
                }
            };

            let now = now();
            match msg {
                Msg::Wake(()) => {
                    while let Some((time, reason)) = self.wake.pop(now) {
                        if let Some(evt) = self.handle_wake(time, reason) {
                            return (evt, now);
                        }
                    }
                },
                Msg::Req((wire_id, req)) => {
                    if let Some(evt) = self.handle_req(now, wire_id, req) {
                        return (evt, now);
                    }
                },
            }
        }
    }

    fn handle_wake(&mut self, now: Time, reason: WakeReason) -> Option<Event> {
        match reason {
            WakeReason::HandleInput(cid, input) =>
                Some(Event::Client(cid, ClientEvent::Input(input))),
            WakeReason::HandleAction(cid, action) =>
                Some(Event::Client(cid, ClientEvent::Action(action))),
            WakeReason::PhysicsUpdate(eid) =>
                Some(Event::Other(OtherEvent::PhysicsUpdate(eid))),
            WakeReason::CheckView(cid) =>
                Some(Event::Client(cid, ClientEvent::CheckView)),
        }
    }

    fn handle_req(&mut self, now: Time, wire_id: WireId, req: Request) -> Option<Event> {
        if wire_id == CONTROL_WIRE_ID {
            self.handle_control_req(now, req)
        } else {
            if let Some(cid) = self.wire_to_client(wire_id) {
                self.handle_client_req(now, cid, req)
            } else {
                self.handle_pre_login_req(now, wire_id, req)
            }
        }
    }

    fn handle_control_req(&mut self, now: Time, req: Request) -> Option<Event> {
        match req {
            Request::AddClient(wire_id) =>
                // Let the caller decide when to actually add the client.
                Some(Event::Control(ControlEvent::AddClient(wire_id))),
            Request::RemoveClient(wire_id) => {
                // Let the caller decide when to actually remove the client.
                let opt_cid = self.wire_to_client(wire_id);
                Some(Event::Control(ControlEvent::RemoveClient(wire_id, opt_cid)))
            },
            Request::ReplCommand(cookie, cmd) =>
                Some(Event::Control(ControlEvent::ReplCommand(cookie, cmd))),

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

    fn handle_client_req(&mut self, now: Time, cid: ClientId, req: Request) -> Option<Event> {
        match self.try_handle_client_req(now, cid, req) {
            Ok(evt) => evt.map(|e| Event::Client(cid, e)),
            Err(e) => {
                warn!("bad request from {:?}: {}", cid, e.description());
                Some(Event::Client(cid, ClientEvent::BadRequest))
            },
        }
    }

    fn try_handle_client_req(&mut self,
                             now: Time,
                             cid: ClientId,
                             req: Request) -> StringResult<Option<ClientEvent>> {
        match req {
            Request::Ping(cookie) => {
                self.send(cid, Response::Pong(cookie, now.to_local()));
                Ok(None)
            },

            Request::Input(time, input) => {
                let time = cmp::max(time.to_global(now), now);
                let input = unwrap!(InputBits::from_bits(input));
                self.wake.push(time, WakeReason::HandleInput(cid, input));
                Ok(None)
            },

            Request::Action(time, action, arg) => {
                let time = cmp::max(time.to_global(now), now);
                let action = unwrap!(Action::decode(action, arg));
                self.wake.push(time, WakeReason::HandleAction(cid, action));
                Ok(None)
            },

            Request::UnsubscribeInventory(iid) =>
                Ok(Some(ClientEvent::UnsubscribeInventory(iid))),

            Request::MoveItem(from_iid, to_iid, item_id, count) =>
                Ok(Some(ClientEvent::MoveItem(from_iid, to_iid, item_id, count))),

            Request::CraftRecipe(sid, iid, recipe_id, count) =>
                Ok(Some(ClientEvent::CraftRecipe(sid, iid, recipe_id, count))),

            Request::Chat(msg) =>
                Ok(Some(ClientEvent::Chat(msg))),

            _ => fail!("bad request: {:?}", req),
        }
    }


    pub fn wire_to_client(&self, wire: WireId) -> Option<ClientId> {
        unimplemented!()
    }

    pub fn client_to_wire(&self, cid: ClientId) -> Option<WireId> {
        unimplemented!()
    }

    fn send_raw(&self, wire_id: WireId, msg: Response) {
        self.send.send((wire_id, msg)).unwrap();
    }

    pub fn send_control(&self, resp: ControlResponse) {
        match resp {
            ControlResponse::ReplResult(cookie, msg) =>
                self.send_raw(CONTROL_WIRE_ID, Response::ReplResult(cookie, msg)),
        }
    }

    pub fn send_wire(&self, wire_id: WireId, resp: WireResponse) {
        match resp {
            WireResponse::RegisterResult(code, msg) =>
                self.send_raw(wire_id, Response::RegisterResult(code, msg)),
        }
    }

    pub fn send_client(&self, cid: ClientId, resp: ClientResponse) {
        let wire_id = match self.client_to_wire(cid) {
            Some(x) => x,
            None => {
                warn!("can't send to client {:?} (no wire): {:?}", cid, resp);
                return;
            },
        };

        match resp {
            ClientResponse::TerrainChunk(cpos, data) => {
                let index = unimplemented!();
                self.send_raw(wire_id, Response::TerrainChunk(index, data));
            },

            ClientResponse::UnloadChunk(cpos) => {
                let index = unimplemented!();
                self.send_raw(wire_id, Response::UnloadChunk(index));
            },

            ClientResponse::EntityAppear(eid, appear, name) =>
                self.send_raw(wire_id, Response::EntityAppear(eid, appear, name)),

            ClientResponse::EntityUpdate(eid, motion, anim) => {
                let wire_motion = unimplemented!();
                self.send_raw(wire_id, Response::EntityUpdate(eid, wire_motion, anim));
            },

            ClientResponse::EntityGone(eid, time) => {
                let time = unimplemented!();
                self.send_raw(wire_id, Response::EntityGone(eid, time));
            },

            ClientResponse::InventoryUpdate(iid, update) =>
                self.send_raw(wire_id, Response::InventoryUpdate(iid, update)),

            ClientResponse::OpenDialog(dialog) => {
                match dialog {
                    Dialog::Inventory(iid) => 
                        self.send_raw(wire_id, Response::OpenDialog(0, vec![iid.unwrap()])),
                    Dialog::Container(iid1, iid2) => 
                        self.send_raw(wire_id, Response::OpenDialog(0, vec![iid1.unwrap(),
                                                                            iid2.unwrap()])),
                    Dialog::Crafting(template_id, sid, iid) =>
                        self.send_raw(wire_id, Response::OpenCrafting(template_id, sid, iid)),
                }
            },

            ClientResponse::ChatUpdate(msg) =>
                self.send_raw(wire_id, Response::ChatUpdate(msg)),

            ClientResponse::KickReason(msg) =>
                self.send_raw(wire_id, Response::KickReason(msg)),
        }
    }

    pub fn send(&self, cid: ClientId, msg: Response) {
        unimplemented!()
    }
}


fn now() -> Time {
    let timespec = time::get_time();
    (timespec.sec as Time * 1000) + (timespec.nsec / 1000000) as Time
}
