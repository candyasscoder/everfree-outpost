use std::collections::{HashMap, hash_map};
use rand::{self, Rng};

use physics::{CHUNK_SIZE, CHUNK_BITS, TILE_SIZE, TILE_BITS};

use types::*;

use msg;
use world;


pub struct Clients {
    clients: HashMap<ClientId, ClientInfo>,
    wire_map: HashMap<WireId, ClientId>,
}

pub struct ClientInfo {
    wire_id: WireId,
    chunk_offset: (u8, u8),
    last_check: Time,
}

impl Clients {
    pub fn new() -> Clients {
        Clients {
            clients: HashMap::new(),
            wire_map: HashMap::new(),
        }
    }

    pub fn add(&mut self, cid: ClientId, wire_id: WireId) {
        let old_client = self.clients.insert(cid, ClientInfo::new(wire_id));
        let old_wire = self.wire_map.insert(wire_id, cid);
        debug_assert!(old_client.is_none());
        debug_assert!(old_wire.is_none());
    }

    pub fn remove(&mut self, cid: ClientId) {
        let info = self.clients.remove(&cid).expect("client does not exist");
        self.wire_map.remove(&info.wire_id).expect("client was not in wire_map");
    }

    pub fn wire_to_client(&self, wire_id: WireId) -> Option<ClientId> {
        self.wire_map.get(&wire_id).map(|&x| x)
    }

    pub fn get(&self, cid: ClientId) -> Option<&ClientInfo> {
        self.clients.get(&cid)
    }

    pub fn get_mut(&mut self, cid: ClientId) -> Option<&mut ClientInfo> {
        self.clients.get_mut(&cid)
    }

    pub fn iter(&self) -> hash_map::Iter<ClientId, ClientInfo> {
        self.clients.iter()
    }

    pub fn len(&self) -> usize {
        self.clients.len()
    }
}

const LOCAL_BITS: usize = 3;
const LOCAL_SIZE: i32 = 1 << LOCAL_BITS;
const LOCAL_MASK: i32 = LOCAL_SIZE - 1;

impl ClientInfo {
    pub fn new(wire_id: WireId) -> ClientInfo {
        let mut rng = rand::thread_rng();
        let offset_x = rng.gen_range(0, 8);
        let offset_y = rng.gen_range(0, 8);
        info!("offset: {} {}", offset_x, offset_y);
        ClientInfo {
            wire_id: wire_id,
            chunk_offset: (offset_x, offset_y),
            last_check: TIME_MIN,
        }
    }

    pub fn wire_id(&self) -> WireId {
        self.wire_id
    }

    pub fn local_chunk_index(&self, cpos: V2) -> u16 {
        let cx = (cpos.x + self.chunk_offset.0 as i32) & LOCAL_MASK;
        let cy = (cpos.y + self.chunk_offset.1 as i32) & LOCAL_MASK;
        (cy * LOCAL_SIZE + cx) as u16
    }

    pub fn local_pos(&self, pos: V3) -> V3 {
        const MASK: i32 = (1 << (TILE_BITS + CHUNK_BITS + LOCAL_BITS)) - 1;
        let x = (pos.x + self.chunk_offset.0 as i32 * CHUNK_SIZE * TILE_SIZE) & MASK;
        let y = (pos.y + self.chunk_offset.1 as i32 * CHUNK_SIZE * TILE_SIZE) & MASK;
        let z = pos.z;
        V3::new(x, y, z)
    }

    pub fn local_motion(&self, m: world::Motion) -> msg::Motion {
        let base = TILE_SIZE * CHUNK_SIZE * LOCAL_SIZE;
        let start = self.local_pos(m.start_pos) + V3::new(base, base, 0);
        let end = start + (m.end_pos - m.start_pos);

        msg::Motion {
            start_pos: (start.x as u16,
                        start.y as u16,
                        start.z as u16),
            start_time: m.start_time.to_local(),

            end_pos: (end.x as u16,
                      end.y as u16,
                      end.z as u16),
            end_time: (m.start_time + m.duration as Time).to_local(),
        }
    }

    pub fn maybe_check(&mut self, now: Time) -> bool {
        if now < self.last_check + 1000 {
            false
        } else {
            self.last_check = now;
            true
        }
    }
}
