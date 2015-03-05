use std::collections::HashMap;
use std::rand;

use physics::{CHUNK_SIZE, CHUNK_BITS};

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
}

const LOCAL_BITS: usize = 3;
const LOCAL_SIZE: i32 = 1 << LOCAL_BITS;
const LOCAL_MASK: i32 = LOCAL_SIZE - 1;

impl ClientInfo {
    pub fn new(wire_id: WireId) -> ClientInfo {
        ClientInfo {
            wire_id: wire_id,
            chunk_offset: rand::random(),
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
        const MASK: i32 = (1 << (CHUNK_BITS + LOCAL_BITS)) - 1;
        let x = (pos.x + self.chunk_offset.0 as i32 * CHUNK_SIZE) & MASK;
        let y = (pos.y + self.chunk_offset.1 as i32 * CHUNK_SIZE) & MASK;
        let z = pos.z;
        V3::new(x, y, z)
    }

    pub fn local_motion(&self, m: world::Motion) -> msg::Motion {
        let V3 { x: sx, y: sy, z: sz } = self.local_pos(m.start_pos);
        let V3 { x: ex, y: ey, z: ez } = self.local_pos(m.end_pos);

        msg::Motion {
            start_pos: (sx as u16,
                        sy as u16,
                        sz as u16),
            start_time: m.start_time.to_local(),

            end_pos: (ex as u16,
                      ey as u16,
                      ez as u16),
            end_time: (m.start_time + m.duration as Time).to_local(),
        }
    }
}
