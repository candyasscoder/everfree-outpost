use std::borrow::IntoCow;
use std::error::Error;
use std::sync::mpsc::{Sender, Receiver};

use types::*;
use util::Cursor;

use auth::{Auth, Secret};
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
use vision::Vision;
use world::World;


pub struct Engine<'d> {
    pub data: &'d Data,
    pub storage: &'d Storage,

    pub world: World<'d>,
    pub script: ScriptEngine,

    // any update
    pub messages: Messages,
    // terrain or structure change
    pub physics: Physics<'d>,
    // any update
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
                match self.auth.login(&*name, &secret) {
                    Ok(true) => {
                        info!("{:?}: logged in as {}", wire_id, name);
                        unimplemented!()
                    },
                    Ok(false) => {
                        info!("{:?}: login as {} failed: bad name/secret",
                              wire_id, name);
                        self.kick_wire(wire_id, "login failed")
                    },
                    Err(e) => {
                        info!("{:?}: login as {} failed: auth error: {}",
                              wire_id, name, e.description());
                        self.kick_wire(wire_id, "login failed")
                    },
                }
            },

            Register(name, secret, appearance) => {
                let (code, msg) = self.do_register(wire_id, name, secret, appearance);
                self.messages.send_wire(wire_id, RegisterResult(code, msg));
            },

            BadRequest => {
                self.kick_wire(wire_id, "bad request");
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
                self.kick_client(cid, "bad request");
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

    fn kick_client<'a, S: IntoCow<'a, str>>(&mut self, cid: ClientId, msg: S) {
        let wire_id = self.messages.client_to_wire(cid)
                .expect("missing WireId for existing client");

        self.messages.send_client(cid, ClientResponse::KickReason(msg.into_cow().into_owned()));
        self.cleanup_client(cid);
        self.messages.send_control(ControlResponse::WireClosed(wire_id));
    }

    fn kick_wire<'a, S: IntoCow<'a, str>>(&mut self, wire_id: WireId, msg: S) {
        self.messages.send_wire(wire_id, WireResponse::KickReason(msg.into_cow().into_owned()));
        self.cleanup_wire(wire_id);
        self.messages.send_control(ControlResponse::WireClosed(wire_id));
    }


    fn do_register(&mut self,
                   wire_id: WireId,
                   name: String,
                   secret: Secret,
                   appearance: u32) -> (u32, String) {
        if let Err(msg) = name_valid(&*name) {
            return (1, String::from_str(msg));
        }

        match self.auth.register(&*name, &secret) {
            Ok(true) => {
                info!("{:?}: registered as {}", wire_id, name);
                unimplemented!()
            },
            Ok(false) => {
                info!("{:?}: registration as {} failed: name is in use",
                      wire_id, name);
                (1, String::from_str("That name is already in use."))
            },
            Err(e) => {
                info!("{:?}: registration as {} failed: database error: {}",
                      wire_id, name, e.description());
                (1, String::from_str("An internal error occurred."))
            }
        }
    }
}

fn name_valid(name: &str) -> Result<(), &'static str> {
    if name.len() == 0 {
        return Err("Please enter a name.");
    }

    if name.len() > 16 {
        return Err("Name is too long (must not exceed 16 characters).");
    }

    let chars_ok = name.chars().all(|c| {
        (c >= 'a' && c <= 'z') ||
        (c >= 'A' && c <= 'Z') ||
        (c >= '0' && c <= '9') ||
        c == ' ' ||
        c == '-'
    });
    if !chars_ok {
        return Err("Names may only contain letters, numbers, spaces, and hyphens.");
    }

    let has_alnum = name.chars().all(|c| {
        (c >= 'a' && c <= 'z') ||
        (c >= 'A' && c <= 'Z') ||
        (c >= '0' && c <= '9')
    });
    if !has_alnum {
        return Err("Names must contain at least one letter or digit.");
    }

    Ok(())
}
