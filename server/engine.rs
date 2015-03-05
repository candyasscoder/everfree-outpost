use std::sync::mpsc::{Sender, Receiver};

use types::*;
use util::Cursor;

use auth::Auth;
use chunks::Chunks;
use data::Data;
use messages::{Messages};
use messages::{Event, ControlEvent, WireEvent, ClientEvent, OtherEvent};
use messages::{ControlResponse, WireResponse, ClientResponse};
use msg::{Request, Response};
use physics_::Physics;
use script::ScriptEngine;
use storage::Storage;
use terrain_gen::TerrainGen;
use view::Vision;
use world::World;


pub struct Engine<'d> {
    pub data: &'d Data,
    pub storage: &'d Storage,

    pub world: World<'d>,
    pub script: ScriptEngine,

    pub messages: Messages,
    pub physics: Physics<'d>,
    pub vision: Vision,
    pub auth: Auth,
    pub chunks: Chunks<'d>,
    pub terrain_gen: TerrainGen<'d>,
}

impl<'d> Engine<'d> {
    pub fn new(data: &'d Data,
           storage: &'d Storage,
           receiver: Receiver<(WireId, Request)>,
           sender: Sender<(WireId, Response)>) -> Engine<'d> {
        Engine {
            data: data,
            storage: storage,

            world: World::new(data),
            script: ScriptEngine::new(&storage.script_dir()),

            messages: Messages::new(receiver, sender),
            physics: Physics::new(data),
            vision: Vision::new(),
            auth: Auth::new(storage.auth_db_path()).unwrap(),
            chunks: Chunks::new(storage),
            terrain_gen: TerrainGen::new(data),
        }
    }

    pub fn run(&mut self) -> ! {
        loop {
            let (evt, now) = self.messages.next_event();
            self.handle(now, evt);
            /*
            self.vision.finish(&self.world,
                               &self.events);
                               */
        }
    }


    fn handle(&mut self,
              now: Time,
              evt: Event) {
        use messages::Event::*;
        match evt {
            Control(e) => self.handle_control(now, e),
            Wire(wire_id, e) => self.handle_wire(now, wire_id, e),
            Client(cid, e) => self.handle_client(now, cid, e),
            Other(e) => self.handle_other(now, e),
        }
    }

    fn handle_control(&mut self,
                      now: Time,
                      evt: ControlEvent) {
        use messages::ControlEvent::*;
        use messages::ControlResponse::*;
        match evt {
            OpenWire(wire_id) => {},

            CloseWire(wire_id, opt_cid) => {
                if let Some(cid) = opt_cid {
                    self.cleanup_client(cid);
                }
                self.messages.send_control(WireClosed(wire_id));
            },

            ReplCommand(cookie, msg) => {
                unimplemented!();
            },
        }
    }

    fn handle_wire(&mut self,
                   now: Time,
                   wire_id: WireId,
                   evt: WireEvent) {
        use messages::WireEvent::*;
        use messages::WireResponse::*;
        match evt {
            Login(name, secret) => {
                unimplemented!();
            },

            Register(name, secret, appearance) => {
                unimplemented!();
            },

            BadRequest => {
                let msg = String::from_str("bad request");
                self.messages.send_wire(wire_id, KickReason(msg));
                self.cleanup_wire(wire_id);
                self.messages.send_control(ControlResponse::WireClosed(wire_id));
            },
        }
    }

    fn handle_client(&mut self,
                     now: Time,
                     cid: ClientId,
                     evt: ClientEvent) {
        use messages::ClientEvent::*;
        use messages::ClientResponse::*;
        match evt {
            Input(input) => {
                unimplemented!()
            },

            Action(action) => {
                unimplemented!()
            },

            UnsubscribeInventory(iid) => {
                unimplemented!()
            },

            MoveItem(from_iid, to_iid, item_id, count) => {
                unimplemented!()
            },

            CraftRecipe(station_sid, iid, recipe_id, count) => {
                unimplemented!()
            },

            Chat(msg) => {
                unimplemented!()
            },

            CheckView => {
                unimplemented!()
            },

            BadRequest => {
                let wire_id = self.messages.client_to_wire(cid)
                        .expect("missing WireId for existing client");

                let msg = String::from_str("bad request");
                self.messages.send_client(cid, KickReason(msg));
                self.cleanup_client(cid);
                self.messages.send_control(ControlResponse::WireClosed(wire_id));
            },
        }
    }

    fn handle_other(&mut self,
                    now: Time,
                    evt: OtherEvent) {
        use messages::OtherEvent::*;
        match evt {
            PhysicsUpdate(eid) => {
                unimplemented!();
            },
        }
    }


    fn cleanup_client(&mut self, cid: ClientId) {
        self.messages.remove_client(cid);
    }

    fn cleanup_wire(&mut self, wire_id: WireId) {
        if let Some(cid) = self.messages.wire_to_client(wire_id) {
            self.cleanup_client(cid);
        }
    }
}
