use std::boxed::FnBox;
use std::error::Error;
use std::sync::mpsc::{Sender, Receiver};

use types::*;

use auth::{Auth, Secret};
use cache::TerrainCache;
use chunks::Chunks;
use data::Data;
use logic;
use logic::extra::Extra;
use messages::{Messages, MessageEvent};
use messages::{Event, ControlEvent, WireEvent, ClientEvent};
use messages::SyncKind;
use messages::{ControlResponse, WireResponse, ClientResponse};
use msg::{Request, Response};
use physics::Physics;
use script::ScriptEngine;
use storage::Storage;
use terrain_gen::{TerrainGen, TerrainGenEvent};
use terrain_gen::Fragment as TerrainGen_Fragment;
use timer::{Timer, TimerEvent};
use vision::Vision;
use world::World;

use self::split::EngineRef;


#[macro_use] pub mod split;
pub mod glue;


pub struct Engine<'d> {
    pub data: &'d Data,
    pub storage: &'d Storage,
    pub now: Time,

    pub world: World<'d>,
    pub script: ScriptEngine,

    pub extra: Extra,
    pub messages: Messages,
    pub timer: Timer,
    pub physics: Physics<'d>,
    pub vision: Vision,
    pub auth: Auth,
    pub chunks: Chunks<'d>,
    pub cache: TerrainCache,
    pub terrain_gen: TerrainGen<'d>,
}

#[must_use]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum HandlerResult {
    Continue,
    Shutdown,
    Restart,
}

impl<'d> Engine<'d> {
    pub fn new(data: &'d Data,
           storage: &'d Storage,
           receiver: Receiver<(WireId, Request)>,
           sender: Sender<(WireId, Response)>) -> Engine<'d> {
        Engine {
            data: data,
            storage: storage,
            now: TIME_MIN,

            world: World::new(data),
            script: ScriptEngine::new(&storage.script_dir()),

            extra: Extra::new(),
            messages: Messages::new(receiver, sender),
            timer: Timer::new(),
            physics: Physics::new(data),
            vision: Vision::new(),
            auth: Auth::new(&storage.auth_db_path()).unwrap(),
            chunks: Chunks::new(storage),
            cache: TerrainCache::new(),
            terrain_gen: TerrainGen::new(data, storage),
        }
    }

    pub fn run(&mut self) {
        use self::HandlerResult::*;
        logic::lifecycle::start_up(self.as_ref());
        if let Some(file) = self.storage.open_restart_file() {
            logic::lifecycle::post_restart(self.as_ref(), file);
            self.storage.remove_restart_file();
        }

        loop {
            enum Event {
                FromTimer(TimerEvent),
                FromMessage(MessageEvent),
                FromTerrainGen(TerrainGenEvent),
            }

            let evt = {
                let recv_timer = self.timer.receiver();
                let recv_message = self.messages.receiver();
                let recv_terrain_gen = self.terrain_gen.receiver();
                select! {
                    evt = recv_timer.recv() => Event::FromTimer(evt.unwrap()),
                    evt = recv_message.recv() => Event::FromMessage(evt.unwrap()),
                    evt = recv_terrain_gen.recv() => Event::FromTerrainGen(evt.unwrap())
                }
            };

            match evt {
                Event::FromTimer(evt) => {
                    let (cb, now) = unwrap_or!(self.timer.process(evt), continue);
                    self.now = now;
                    cb.call_box((self.as_ref(),));
                },
                Event::FromMessage(evt) => {
                    let (evt, now) = unwrap_or!(self.messages.process(evt), continue);
                    match self.handle(now, evt) {
                        Continue => {},
                        Shutdown => break,
                        Restart => {
                            logic::lifecycle::pre_restart(self.as_ref());
                            break;
                        },
                    }
                },
                Event::FromTerrainGen(evt) => {
                    self.as_ref().as_terrain_gen_fragment().process(evt);
                },
            }
        }

        logic::lifecycle::shut_down(self.as_ref());
    }


    fn handle(&mut self,
              now: Time,
              evt: Event) -> HandlerResult {
        use messages::Event::*;
        self.now = now;
        match evt {
            Control(e) => self.handle_control(e),
            Wire(wire_id, e) => self.handle_wire(wire_id, e),
            Client(cid, e) => self.handle_client(cid, e),
        }
    }

    fn handle_control(&mut self,
                      evt: ControlEvent) -> HandlerResult {
        use messages::ControlEvent::*;
        use messages::ControlResponse::*;
        match evt {
            OpenWire(_wire_id) => {},

            CloseWire(wire_id, opt_cid) => {
                if let Some(cid) = opt_cid {
                    self.cleanup_client(cid);
                }
                self.messages.send_control(WireClosed(wire_id));
            },

            ReplCommand(cookie, msg) => {
                match ScriptEngine::cb_eval(self, &*msg) {
                    Ok(result) => self.messages.send_control(ReplResult(cookie, result)),
                    Err(e) => {
                        warn!("eval error: {}", e);
                        let resp = ReplResult(cookie, String::from("eval error"));
                        self.messages.send_control(resp);
                    },
                }
            },

            Shutdown => {
                return HandlerResult::Shutdown;
            },

            Restart(server, client) => {
                if client {
                    self.messages.broadcast_clients(ClientResponse::SyncStatus(SyncKind::Refresh));
                }
                if server {
                    return HandlerResult::Restart;
                }
            },
        }
        HandlerResult::Continue
    }

    fn handle_wire(&mut self,
                   wire_id: WireId,
                   evt: WireEvent) -> HandlerResult {
        use messages::WireEvent::*;
        use messages::WireResponse::*;
        match evt {
            Login(name, secret) => {
                match self.auth.login(&*name, &secret) {
                    Ok(true) => {
                        warn_on_err!(logic::client::login(self.as_ref(), wire_id, &*name));
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
        HandlerResult::Continue
    }

    fn handle_client(&mut self,
                     cid: ClientId,
                     evt: ClientEvent) -> HandlerResult {
        use messages::ClientEvent::*;
        match evt {
            Input(time, input) => {
                self.timer.schedule(time,
                                    move |eng| logic::input::input(eng, cid, input));
            },

            UnsubscribeInventory(iid) => {
                logic::input::unsubscribe_inventory(self.as_ref(), cid, iid);
            },

            MoveItem(from_iid, from_slot, to_iid, to_slot, count) => {
                warn_on_err!(logic::items::move_items2(self.as_ref(),
                                                       from_iid,
                                                       from_slot,
                                                       to_iid,
                                                       to_slot,
                                                       count));
            },

            CraftRecipe(station_sid, iid, recipe_id, count) => {
                warn_on_err!(logic::items::craft_recipe(self.as_ref(),
                                                        station_sid, iid, recipe_id, count));
            },

            Chat(msg) => {
                logic::input::chat(self.as_ref(), cid, msg);
            },

            Interact(time, args) => {
                self.timer.schedule(time,
                                    move |eng| logic::input::interact(eng, cid, args));
            },

            UseItem(time, item_id, args) => {
                self.timer.schedule(time,
                                    move |eng| logic::input::use_item(eng, cid, item_id, args));
            },

            UseAbility(time, item_id, args) => {
                self.timer.schedule(time,
                                    move |eng| logic::input::use_ability(eng, cid, item_id, args));
            },

            BadRequest => {
                self.kick_client(cid, "bad request");
            },
        }
        HandlerResult::Continue
    }


    fn cleanup_client(&mut self, cid: ClientId) {
        warn_on_err!(logic::client::logout(self.as_ref(), cid));
    }

    fn cleanup_wire(&mut self, wire_id: WireId) {
        if let Some(cid) = self.messages.wire_to_client(wire_id) {
            self.cleanup_client(cid);
        }
    }

    pub fn kick_client<'a, S: Into<String>>(&mut self, cid: ClientId, msg: S) {
        let wire_id = self.messages.client_to_wire(cid)
                .expect("missing WireId for existing client");

        self.messages.send_client(cid, ClientResponse::KickReason(msg.into()));
        self.cleanup_client(cid);
        self.messages.send_control(ControlResponse::WireClosed(wire_id));
    }

    pub fn kick_wire<'a, S: Into<String>>(&mut self, wire_id: WireId, msg: S) {
        self.messages.send_wire(wire_id, WireResponse::KickReason(msg.into()));
        self.cleanup_wire(wire_id);
        self.messages.send_control(ControlResponse::WireClosed(wire_id));
    }


    pub fn as_ref<'b>(&'b mut self) -> EngineRef<'b, 'd> {
        EngineRef::new(self)
    }


    fn do_register(&mut self,
                   wire_id: WireId,
                   name: String,
                   secret: Secret,
                   appearance: u32) -> (u32, String) {
        if let Err(msg) = name_valid(&*name) {
            return (1, String::from(msg));
        }

        match self.auth.register(&*name, &secret) {
            Ok(true) => {
                info!("{:?}: registered as {}", wire_id, name);
                match logic::client::register(self.as_ref(), &*name, appearance) {
                    Ok(()) => (0, String::new()),
                    Err(e) => {
                        warn!("{:?}: error registering as {}: {}",
                              wire_id, name, e.description());
                        (2, String::from("An internal error occurred."))
                    }
                }
            },
            Ok(false) => {
                info!("{:?}: registration as {} failed: name is in use",
                      wire_id, name);
                (1, String::from("That name is already in use."))
            },
            Err(e) => {
                info!("{:?}: registration as {} failed: database error: {}",
                      wire_id, name, e.description());
                (2, String::from("An internal error occurred."))
            }
        }
    }

    pub fn now(&self) -> Time {
        self.now
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

    let has_alnum = name.chars().any(|c| {
        (c >= 'a' && c <= 'z') ||
        (c >= 'A' && c <= 'Z') ||
        (c >= '0' && c <= '9')
    });
    if !has_alnum {
        return Err("Names must contain at least one letter or digit.");
    }

    Ok(())
}
