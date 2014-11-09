#![crate_name = "backend"]
#![feature(phase)]
#![feature(tuple_indexing, if_let)]
#![feature(unboxed_closures, overloaded_calls)]
#![feature(macro_rules)]
#![allow(non_upper_case_globals)]

#[phase(plugin, link)]
extern crate log;
extern crate time;

extern crate physics;

use std::collections::HashMap;
use std::io;
use std::io::{BufReader, BufWriter};
use std::io::IoResult;
use std::mem;
use std::rand::{StdRng, Rng};
use std::u16;

use physics::{CHUNK_SIZE, CHUNK_BITS, TILE_SIZE};
use physics::v3::{V3, scalar};

use wire::{WireReader, WireWriter};
use msg::Motion as WireMotion;
use msg::{Request, Response, ClientId};
use state::{EntityId, InputBits};

mod msg;
mod wire;
mod tasks;
mod state;
mod timer;

pub type Time = u16;

fn main() {
    let (req_send, req_recv) = channel();
    let (resp_send, resp_recv) = channel();

    spawn(proc() {
        let reader = io::stdin();
        tasks::run_input(reader, req_send).unwrap();
    });

    spawn(proc() {
        let writer = io::BufferedWriter::new(io::stdout().unwrap());
        tasks::run_output(writer, resp_recv).unwrap();
    });

    real_main(req_recv, resp_send)
}


fn real_main(reqs: Receiver<(ClientId, Request)>,
             resps: Sender<(ClientId, Response)>) {
    let mut state = state::State::new();
    state.init_terrain();

    loop {
        let wake_recv = state.wake_queue.wait_recv();

        select! {
            () = wake_recv.recv() => {
                while let Some((time, reason)) = state.wake_queue.pop() {
                    handle_wake(&mut state,
                                &resps,
                                time,
                                reason);
                }
            },

            (id, req) = reqs.recv() => {
                handle_req(&mut state, &resps, id, req);
            }
        }
    }
}

fn handle_req(state: &mut state::State,
              resps: &Sender<(ClientId, Response)>,
              id: ClientId,
              req: Request) {
    match req {
        msg::GetTerrain => {
            warn!("client {} used deprecated opcode GetTerrain", id);
        },

        msg::UpdateMotion(wire_motion) => {
            warn!("client {} used deprecated opcode UpdateMotion", id);
        },

        msg::Ping(cookie) => {
            resps.send((id, msg::Pong(cookie, now())));
        },

        msg::Input(time, input) => {
            let now = now();
            let input = InputBits::from_bits_truncate(input);
            let updated = state.update_input(now, id, input);
            if updated {
                let (entity_id, motion, anim) = {
                    let ce = state.client_entity(id).unwrap();
                    (ce.client.entity_id,
                     entity_motion(now, ce),
                     ce.entity.anim)
                };
                for &send_id in state.clients.keys() {
                    resps.send((send_id, msg::EntityUpdate(entity_id, motion, anim)));
                }
            }
        },

        msg::Login(secret, name) => {
            log!(10, "login request for {}", name);
            state.add_client(now(), id);

            let info = msg::InitData {
                entity_id: id as EntityId,
                camera_pos: (0, 0),
                chunks: 8 * 8,
                entities: 1,
            };
            resps.send((id, msg::Init(info)));

            for c in range(0, 8 * 8) {
                let data = state.get_terrain_rle16(c);
                resps.send((id, msg::TerrainChunk(c as u16, data)));
            }

            let ce = state.client_entity(id).unwrap();
            let motion = entity_motion(now(), ce);
            let anim = ce.entity.anim;
            resps.send((id, msg::EntityUpdate(ce.client.entity_id, motion, anim)));
        },

        msg::AddClient => {
            //state.add_client(now(), id);
        },

        msg::RemoveClient => {
            state.remove_client(id);
            resps.send((id, msg::ClientRemoved));
        },

        msg::BadMessage(opcode) => {
            warn!("unrecognized opcode from client {}: {:x}",
                  id, opcode.unwrap());
        },
    }
}

fn handle_wake(state: &mut state::State,
               resps: &Sender<(ClientId, Response)>,
               time: i64,
               reason: state::WakeReason) {
    warn!("unimplemented: handle_wake");
}

fn now() -> u16 {
    let timespec = time::get_time();
    (timespec.sec as u16 * 1000) + (timespec.nsec / 1000000) as u16
}

fn entity_motion(now: Time, ce: state::ClientEntity) -> WireMotion {
    let pos = ce.entity.pos(now);
    let world_base = state::base_chunk(pos);
    let local_base = state::offset_base_chunk(world_base, ce.client.chunk_offset);

    let start_pos = state::world_to_local(ce.entity.start_pos, world_base, local_base);
    let end_pos = state::world_to_local(ce.entity.end_pos, world_base, local_base);
    
    WireMotion {
        start_time: ce.entity.start_time,
        end_time: ce.entity.start_time + ce.entity.duration,
        start_pos: (start_pos.x as u16,
                    start_pos.y as u16,
                    start_pos.z as u16),
        end_pos: (end_pos.x as u16,
                    end_pos.y as u16,
                    end_pos.z as u16),
    }
}
