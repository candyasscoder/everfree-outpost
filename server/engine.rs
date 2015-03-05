use std::sync::mpsc::{Sender, Receiver};

use types::*;
use util::Cursor;

use auth::Auth;
use chunks::Chunks;
use data::Data;
use events::{Events, Event, WakeReason};
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

    pub events: Events,
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

            events: Events::new(receiver, sender),
            physics: Physics::new(data),
            vision: Vision::new(),
            auth: Auth::new(storage.auth_db_path()).unwrap(),
            chunks: Chunks::new(storage),
            terrain_gen: TerrainGen::new(data),
        }
    }

    pub fn run(&mut self) -> ! {
        loop {
            let (evt, now) = self.events.next();
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
        /*
        match evt {
            Event::Request(sender_id, req) => self.handle_request(now, sender_id, req),
            Event::Wakeup(cid, wake) => self.handle_wakeup(now, cid, wake),
        }
        */
    }

    /*
    fn handle_request(&mut self,
                      now: Time,
                      sender_id: SenderId,
                      req: Request) {
        /*
        match sender_id {
            SenderId::Control => self.handle_request_control(now, req),
            SenderId::Wire(wire_id) => self.handle_request_pre_login(now, wire_id, req),
            SenderId::Client(cid) => self.handle_request_client(now, cid, req),
        }
        */
    }
    */

    fn handle_request_control(&mut self,
                              now: Time,
                              req: Request) {
        /*
        match req {
            Request::AddClient(_) => {},

            Request::RemoveClient(wire_id) => {
                if let Some(client_id) = self.events.wire_to_client(wire_id) {
                    self.cleanup_client(now, wire_id, client_id);
                }
                self.events.send_control(Response::ClientRemoved(wire_id));
            },

            /*
            Request::ReplCommand(cookie, cmd) => {
                info!("got repl command {}: {:?}", cookie, cmd);

                let cursor = Cursor::new(self, |e| &mut e.script);
                let result = match ScriptEngine::script_eval(cursor, now, &*cmd) {
                    Ok(msg) => msg,
                    Err(e) => e,
                };
                self.events.send_control(Response::ReplResult(cookie, result));
            },
            */

            _ => warn!("bad control request: {:?}", req),
        }
        */
    }

    fn handle_request_pre_login(&mut self,
                                now: Time,
                                wire_id: WireId,
                                req: Request) {
        unimplemented!()
    }

    fn handle_request_client(&mut self,
                             now: Time,
                             cid: ClientId,
                             req: Request) {
        unimplemented!()
    }

    fn handle_wakeup(&mut self,
                     now: Time,
                     cid: ClientId,
                     wake: WakeReason) {
        unimplemented!()
    }


    fn cleanup_client(&mut self, now: Time, wire_id: WireId, cid: ClientId) {
        unimplemented!()
    }
}
