use std::io::IoResult;

use wire;
use wire::{WireReader, WireWriter};
use types::{LocalTime, EntityId};


pub type ClientId = u16;


#[deriving(PartialEq, Eq, Show)]
struct Opcode(u16);

impl Opcode {
    pub fn unwrap(self) -> u16 {
        let Opcode(v) = self;
        v
    }
}

impl wire::WriteTo for Opcode {
    fn write_to<W: Writer>(&self, w: &mut W) -> IoResult<()> {
        self.unwrap().write_to(w)
    }

    fn size(&self) -> uint { self.unwrap().size() }
}

impl wire::WriteToFixed for Opcode {
    fn size_fixed(_: Option<Opcode>) -> uint {
        wire::WriteToFixed::size_fixed(None::<u16>)
    }
}

macro_rules! opcodes {
    ($($name:ident = $value:expr,)*) => {
        $(
            #[allow(non_upper_case_globals, dead_code)]
            pub const $name: Opcode = Opcode($value);
        )*
    }
}

pub mod op {
    use super::Opcode;

    opcodes! {
        GetTerrain = 0x0001,
        UpdateMotion = 0x0002,
        Ping = 0x0003,
        Input = 0x0004,
        Login = 0x0005,

        TerrainChunk = 0x8001,
        PlayerMotion = 0x8002,
        Pong = 0x8003,
        EntityUpdate = 0x8004,
        Init = 0x8005,
        KickReason = 0x8006,
        // TODO: EntityAdd, EntityRemove

        AddClient = 0xff00,
        RemoveClient = 0xff01,
        ClientRemoved = 0xff02,
    }
}


pub enum Request {
    // Ordinary requests
    GetTerrain,
    UpdateMotion(Motion),
    Ping(u16),
    Input(LocalTime, u16),
    Login([u32, ..4], String),

    // Control messages
    AddClient,
    RemoveClient,

    // Server-internal messages
    BadMessage(Opcode),
}

impl Request {
    pub fn read_from<R: Reader>(wr: &mut WireReader<R>) -> IoResult<(ClientId, Request)> {
        let id = try!(wr.read_header());
        let opcode = Opcode(try!(wr.read()));

        let req = match opcode {
            op::GetTerrain => GetTerrain,
            op::UpdateMotion => UpdateMotion(try!(wr.read())),
            op::Ping => Ping(try!(wr.read())),
            op::Input => {
                let (a, b): (LocalTime, u16) = try!(wr.read());
                Input(a, b)
            },
            op::Login => {
                let ((a0, a1, a2, a3), b): ((u32, u32, u32, u32), String) = try!(wr.read());
                Login([a0, a1, a2, a3], b)
            },
            op::AddClient => AddClient,
            op::RemoveClient => RemoveClient,
            _ => BadMessage(opcode),
        };

        if !wr.done() {
            Ok((id, BadMessage(opcode)))
        } else {
            Ok((id, req))
        }
    }
}


pub enum Response {
    TerrainChunk(u16, Vec<u16>),
    PlayerMotion(u16, Motion),
    Pong(u16, LocalTime),
    EntityUpdate(EntityId, Motion, u16),
    Init(InitData),
    KickReason(String),

    ClientRemoved,
}

impl Response {
    pub fn write_to<W: Writer>(&self, id: ClientId, ww: &mut WireWriter<W>) -> IoResult<()> {
        try!(match *self {
            TerrainChunk(idx, ref data) =>
                ww.write_msg(id, (op::TerrainChunk, idx, data.as_slice())),
            PlayerMotion(entity, ref motion) =>
                ww.write_msg(id, (op::PlayerMotion, entity, motion)),
            Pong(data, time) =>
                ww.write_msg(id, (op::Pong, data, time)),
            EntityUpdate(entity_id, motion, anim) =>
                ww.write_msg(id, (op::EntityUpdate, entity_id, motion, anim)),
            Init(ref data) =>
                ww.write_msg(id, (op::Init, data.flatten())),
            KickReason(ref msg) =>
                ww.write_msg(id, (op::KickReason, msg)),

            ClientRemoved =>
                ww.write_msg(id, (op::ClientRemoved)),
        })
        ww.flush()
    }
}


pub struct InitData {
    pub entity_id: EntityId,
    pub camera_pos: (u16, u16),
    pub chunks: u8,
    pub entities: u8,
}

impl InitData {
    fn flatten(self) -> (EntityId, (u16, u16), u8, u8) {
        let InitData { entity_id, camera_pos, chunks, entities } = self;
        (entity_id, camera_pos, chunks, entities)
    }
}


#[deriving(Show)]
pub struct Motion {
    pub start_pos: (u16, u16, u16),
    pub start_time: LocalTime,
    pub end_pos: (u16, u16, u16),
    pub end_time: LocalTime,
}

impl wire::ReadFrom for Motion {
    fn read_from<R: Reader>(r: &mut R, bytes: uint) -> IoResult<Motion> {
        let (a, b, c, d): ((u16, u16, u16), LocalTime, (u16, u16, u16), LocalTime) =
                            try!(wire::ReadFrom::read_from(r, bytes));
        Ok(Motion {
            start_pos: a,
            start_time: b,
            end_pos: c,
            end_time: d,
        })
    }

    fn size(_: Option<Motion>) -> (uint, uint) {
        let fixed = 2 * wire::ReadFrom::size(None::<(u16, u16, u16)>).0 +
                    2 * wire::ReadFrom::size(None::<LocalTime>).0;
        (fixed, 0)
    }
}

impl wire::WriteTo for Motion {
    fn write_to<W: Writer>(&self, w: &mut W) -> IoResult<()> {
        try!(self.start_pos.write_to(w));
        try!(self.start_time.write_to(w));
        try!(self.end_pos.write_to(w));
        try!(self.end_time.write_to(w));
        Ok(())
    }

    fn size(&self) -> uint {
        self.start_pos.size() +
        self.start_time.size() +
        self.end_pos.size() +
        self.end_time.size()
    }
}
