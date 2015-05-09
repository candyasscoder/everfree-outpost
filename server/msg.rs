use std::old_io::IoResult;

use wire;
use wire::{WireReader, WireWriter};
use types::*;

pub use self::Request::*;
pub use self::Response::*;


#[derive(Copy, PartialEq, Eq, Debug)]
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

    fn size(&self) -> usize { self.unwrap().size() }
}

impl wire::WriteToFixed for Opcode {
    fn size_fixed(_: Option<Opcode>) -> usize {
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
        // Requests
        Ping = 0x0003,
        Input = 0x0004,
        Login = 0x0005,
        UnsubscribeInventory = 0x0007,
        MoveItem = 0x0008,
        CraftRecipe = 0x0009,
        Chat = 0x000a,
        Register = 0x000b,
        Interact = 0x000c,
        UseItem = 0x000d,
        UseAbility = 0x000e,

        // Deprecated requests
        GetTerrain = 0x0001,
        UpdateMotion = 0x0002,
        Action = 0x0006,
        OpenInventory = 0x000f,

        // Responses
        TerrainChunk = 0x8001,
        Pong = 0x8003,
        EntityUpdate = 0x8004,
        Init = 0x8005,
        KickReason = 0x8006,
        UnloadChunk = 0x8007,
        OpenDialog = 0x8008,
        InventoryUpdate = 0x8009,
        OpenCrafting = 0x800a,
        ChatUpdate = 0x800b,
        EntityAppear = 0x800c,
        EntityGone = 0x800d,
        RegisterResult = 0x800e,
        StructureAppear = 0x800f,
        StructureGone = 0x8010,
        MainInventory = 0x8011,
        AbilityInventory = 0x8012,

        // Deprecated responses
        PlayerMotion = 0x8002,

        // Control messages
        AddClient = 0xff00,
        RemoveClient = 0xff01,
        ClientRemoved = 0xff02,
        ReplCommand = 0xff03,
        ReplResult = 0xff04,
        Shutdown = 0xff05,
        Restart = 0xff06,
    }
}


#[allow(dead_code)]
#[derive(Debug)]
pub enum Request {
    // Ordinary requests
    GetTerrain,
    UpdateMotion(Motion),
    Ping(u16),
    Input(LocalTime, u16),
    Login(String, [u32; 4]),
    Action(LocalTime, u16, u32),
    UnsubscribeInventory(InventoryId),
    MoveItem(InventoryId, InventoryId, ItemId, u16),
    CraftRecipe(StructureId, InventoryId, RecipeId, u16),
    Chat(String),
    Register(String, [u32; 4], u32),
    Interact(LocalTime),
    UseItem(LocalTime, ItemId),
    UseAbility(LocalTime, ItemId),
    OpenInventory,

    // Control messages
    AddClient(WireId),
    RemoveClient(WireId),
    ReplCommand(u16, String),
    Shutdown,

    // Server-internal messages
    BadMessage(Opcode),
}

impl Request {
    pub fn read_from<R: Reader>(wr: &mut WireReader<R>) -> IoResult<(WireId, Request)> {
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
                // Shuffle order since the String must be last on the wire
                let (b, a) = try!(wr.read());
                Login(a, b)
            },
            op::Action => {
                let (a, b, c) = try!(wr.read());
                Action(a, b, c)
            },
            op::UnsubscribeInventory => {
                let a = try!(wr.read());
                UnsubscribeInventory(a)
            },
            op::MoveItem => {
                let (a, b, c, d) = try!(wr.read());
                MoveItem(a, b, c, d)
            },
            op::CraftRecipe => {
                let (a, b, c, d) = try!(wr.read());
                CraftRecipe(a, b, c, d)
            },
            op::Chat => {
                let a = try!(wr.read());
                Chat(a)
            },
            op::Register => {
                // Shuffle order since the String must be last on the wire
                let (b, c, a) = try!(wr.read());
                Register(a, b, c)
            },
            op::Interact => {
                let a = try!(wr.read());
                Interact(a)
            },
            op::UseItem => {
                let (a, b) = try!(wr.read());
                UseItem(a, b)
            },
            op::UseAbility => {
                let (a, b) = try!(wr.read());
                UseAbility(a, b)
            },
            op::OpenInventory => {
                OpenInventory
            },

            op::AddClient => {
                let a = try!(wr.read());
                AddClient(a)
            },
            op::RemoveClient => {
                let a = try!(wr.read());
                RemoveClient(a)
            },
            op::ReplCommand => {
                let (a, b) = try!(wr.read());
                ReplCommand(a, b)
            },
            op::Shutdown => {
                Shutdown
            },
            _ => BadMessage(opcode),
        };

        if !wr.done() {
            Ok((id, BadMessage(opcode)))
        } else {
            Ok((id, req))
        }
    }
}


#[allow(dead_code)]
pub enum Response {
    TerrainChunk(u16, Vec<u16>),
    PlayerMotion(u16, Motion),
    Pong(u16, LocalTime),
    EntityUpdate(EntityId, Motion, u16),
    Init(InitData),
    KickReason(String),
    UnloadChunk(u16),
    OpenDialog(u32, Vec<u32>),
    InventoryUpdate(InventoryId, Vec<(ItemId, u8, u8)>),
    OpenCrafting(TemplateId, StructureId, InventoryId),
    ChatUpdate(String),
    EntityAppear(EntityId, u32, String),
    EntityGone(EntityId, LocalTime),
    RegisterResult(u32, String),
    StructureAppear(StructureId, TemplateId, (u16, u16, u16)),
    StructureGone(StructureId),
    MainInventory(InventoryId),
    AbilityInventory(InventoryId),

    ClientRemoved(WireId),
    ReplResult(u16, String),
}

impl Response {
    pub fn write_to<W: Writer>(&self, id: WireId, ww: &mut WireWriter<W>) -> IoResult<()> {
        try!(match *self {
            TerrainChunk(idx, ref data) =>
                ww.write_msg(id, (op::TerrainChunk, idx, data)),
            PlayerMotion(entity, ref motion) =>
                ww.write_msg(id, (op::PlayerMotion, entity, motion)),
            Pong(data, time) =>
                ww.write_msg(id, (op::Pong, data, time)),
            EntityUpdate(entity_id, ref motion, anim) =>
                ww.write_msg(id, (op::EntityUpdate, entity_id, motion, anim)),
            Init(ref data) =>
                ww.write_msg(id, (op::Init, data.flatten())),
            KickReason(ref msg) =>
                ww.write_msg(id, (op::KickReason, msg)),
            UnloadChunk(idx) =>
                ww.write_msg(id, (op::UnloadChunk, idx)),
            OpenDialog(dialog_id, ref params) =>
                ww.write_msg(id, (op::OpenDialog, dialog_id, params)),
            InventoryUpdate(inventory_id, ref changes) =>
                ww.write_msg(id, (op::InventoryUpdate, inventory_id, changes)),
            OpenCrafting(station_type, station_id, inventory_id) =>
                ww.write_msg(id, (op::OpenCrafting, station_type, station_id, inventory_id)),
            ChatUpdate(ref msg) =>
                ww.write_msg(id, (op::ChatUpdate, msg)),
            EntityAppear(entity_id, appearance, ref name) =>
                ww.write_msg(id, (op::EntityAppear, entity_id, appearance, name)),
            EntityGone(entity_id, time) =>
                ww.write_msg(id, (op::EntityGone, entity_id, time)),
            RegisterResult(code, ref msg) =>
                ww.write_msg(id, (op::RegisterResult, code, msg)),
            StructureAppear(sid, template_id, pos) =>
                ww.write_msg(id, (op::StructureAppear, sid, template_id, pos)),
            StructureGone(sid) =>
                ww.write_msg(id, (op::StructureGone, sid)),
            MainInventory(iid) =>
                ww.write_msg(id, (op::MainInventory, iid)),
            AbilityInventory(iid) =>
                ww.write_msg(id, (op::AbilityInventory, iid)),

            ClientRemoved(wire_id) =>
                ww.write_msg(id, (op::ClientRemoved, wire_id)),
            ReplResult(cookie, ref msg) =>
                ww.write_msg(id, (op::ReplResult, cookie, msg)),
        });
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
    fn flatten(&self) -> (EntityId, (u16, u16), u8, u8) {
        let InitData { entity_id, camera_pos, chunks, entities } = *self;
        (entity_id, camera_pos, chunks, entities)
    }
}


#[derive(Debug, Clone)]
pub struct Motion {
    pub start_pos: (u16, u16, u16),
    pub start_time: LocalTime,
    pub end_pos: (u16, u16, u16),
    pub end_time: LocalTime,
}

impl wire::ReadFrom for Motion {
    fn read_from<R: Reader>(r: &mut R, bytes: usize) -> IoResult<Motion> {
        let (a, b, c, d): ((u16, u16, u16), LocalTime, (u16, u16, u16), LocalTime) =
                            try!(wire::ReadFrom::read_from(r, bytes));
        Ok(Motion {
            start_pos: a,
            start_time: b,
            end_pos: c,
            end_time: d,
        })
    }

    fn size(_: Option<Motion>) -> (usize, usize) {
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

    fn size(&self) -> usize {
        self.start_pos.size() +
        self.start_time.size() +
        self.end_pos.size() +
        self.end_time.size()
    }
}
