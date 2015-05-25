use std::collections::HashMap;
use std::old_io::{IoResult, IoError, IoErrorKind};

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
    fn write_to<W: Writer>(&self, w: &mut WireWriter<W>) -> IoResult<()> {
        self.unwrap().write_to(w)
    }

    fn size(&self) -> usize { self.unwrap().size() }

    fn size_is_fixed() -> bool { true }
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
        InteractWithArgs = 0x0010,
        UseItemWithArgs = 0x0011,
        UseAbilityWithArgs = 0x0012,

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
        PlaneFlags = 0x8013,
        GetInteractArgs = 0x8014,
        GetUseItemArgs = 0x8015,
        GetUseAbilityArgs = 0x8016,

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
    InteractWithArgs(LocalTime, ExtraArg),
    UseItemWithArgs(LocalTime, ItemId, ExtraArg),
    UseAbilityWithArgs(LocalTime, ItemId, ExtraArg),

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
            op::InteractWithArgs => {
                let (a, b) = try!(wr.read());
                InteractWithArgs(a, b)
            },
            op::UseItemWithArgs => {
                let (a, b, c) = try!(wr.read());
                UseItemWithArgs(a, b, c)
            },
            op::UseAbilityWithArgs => {
                let (a, b, c) = try!(wr.read());
                UseAbilityWithArgs(a, b, c)
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
    PlaneFlags(u32),
    GetInteractArgs(u32, ExtraArg),
    GetUseItemArgs(ItemId, u32, ExtraArg),
    GetUseAbilityArgs(ItemId, u32, ExtraArg),

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
            PlaneFlags(flags) =>
                ww.write_msg(id, (op::PlaneFlags, flags)),
            GetInteractArgs(dialog_id, ref args) =>
                ww.write_msg(id, (op::GetInteractArgs, dialog_id, args)),
            GetUseItemArgs(item_id, dialog_id, ref args) =>
                ww.write_msg(id, (op::GetUseItemArgs, item_id, dialog_id, args)),
            GetUseAbilityArgs(item_id, dialog_id, ref args) =>
                ww.write_msg(id, (op::GetUseAbilityArgs, item_id, dialog_id, args)),

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
    pub now: LocalTime,
    pub cycle_base: u32,
    pub cycle_ms: u32,
}

impl InitData {
    fn flatten(&self) -> (EntityId, LocalTime, u32, u32) {
        let InitData { entity_id, now, cycle_base, cycle_ms } = *self;
        (entity_id, now, cycle_base, cycle_ms)
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
    fn read_from<R: Reader>(r: &mut WireReader<R>) -> IoResult<Motion> {
        let (a, b, c, d): ((u16, u16, u16), LocalTime, (u16, u16, u16), LocalTime) =
                            try!(wire::ReadFrom::read_from(r));
        Ok(Motion {
            start_pos: a,
            start_time: b,
            end_pos: c,
            end_time: d,
        })
    }
}

impl wire::WriteTo for Motion {
    fn write_to<W: Writer>(&self, w: &mut WireWriter<W>) -> IoResult<()> {
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

    fn size_is_fixed() -> bool { true }
}


#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub enum SimpleArg {
    Int(i32),
    Str(String),
}

impl SimpleArg {
    fn into_extra_arg(self) -> ExtraArg {
        match self {
            SimpleArg::Int(x) => ExtraArg::Int(x),
            SimpleArg::Str(x) => ExtraArg::Str(x),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ExtraArg {
    Int(i32),
    Str(String),
    List(Vec<ExtraArg>),
    Map(HashMap<SimpleArg, ExtraArg>),
}

impl ExtraArg {
    fn into_simple_arg(self) -> Result<SimpleArg, ExtraArg> {
        match self {
            ExtraArg::Int(x) => Ok(SimpleArg::Int(x)),
            ExtraArg::Str(x) => Ok(SimpleArg::Str(x)),
            e => Err(e),
        }
    }
}


#[derive(Debug, PartialEq, Eq, Copy)]
enum ArgTag {
    Int = 0,
    Str = 1,
    List = 2,
    Map = 3,
}

impl wire::ReadFrom for ArgTag {
    fn read_from<R: Reader>(r: &mut WireReader<R>) -> IoResult<ArgTag> {
        let tag = match try!(r.read::<u8>()) {
            // TODO: figure out how to use `ArgTag::Int as u8` constants
            0 => ArgTag::Int,
            1 => ArgTag::Str,
            2 => ArgTag::List,
            3 => ArgTag::Map,
            x => return Err(IoError {
                kind: IoErrorKind::OtherIoError,
                desc: "bad ArgTag variant",
                detail: Some(format!("tag = {}", x)),
            }),
        };
        Ok(tag)
    }
}

impl wire::WriteTo for ArgTag {
    fn write_to<W: Writer>(&self, w: &mut WireWriter<W>) -> IoResult<()> {
        w.write(*self as u8)
    }

    fn size(&self) -> usize { (*self as u8).size() }

    fn size_is_fixed() -> bool { true }
}


impl wire::ReadFrom for SimpleArg {
    fn read_from<R: Reader>(r: &mut WireReader<R>) -> IoResult<SimpleArg> {
        let arg = try!(r.read::<ExtraArg>());
        let simple = unwrap_or!(arg.into_simple_arg().ok(),
                                return Err(IoError {
                                    kind: IoErrorKind::OtherIoError,
                                    desc: "non-simple SimpleArg",
                                    detail: None,
                                }));
        Ok(simple)
    }
}

impl wire::WriteTo for SimpleArg {
    fn write_to<W: Writer>(&self, w: &mut WireWriter<W>) -> IoResult<()> {
        use self::SimpleArg::*;
        match *self {
            Int(i) => w.write((ArgTag::Int, i)),
            Str(ref s) => w.write((ArgTag::Str, s)),
        }
    }

    fn size(&self) -> usize {
        use self::SimpleArg::*;
        let inner_size = match *self {
            Int(i) => i.size(),
            Str(ref s) => s.size(),
        };
        1 + inner_size
    }

    fn size_is_fixed() -> bool { false }
}


impl wire::ReadFrom for ExtraArg {
    fn read_from<R: Reader>(r: &mut WireReader<R>) -> IoResult<ExtraArg> {
        use self::ExtraArg::*;
        let arg = match try!(r.read()) {
            ArgTag::Int => Int(try!(r.read())),
            ArgTag::Str => Str(try!(r.read())),
            ArgTag::List => List(try!(r.read())),
            ArgTag::Map => Map(try!(r.read())),
        };
        Ok(arg)
    }
}

impl wire::WriteTo for ExtraArg {
    fn write_to<W: Writer>(&self, w: &mut WireWriter<W>) -> IoResult<()> {
        use self::ExtraArg::*;
        match *self {
            Int(i) => w.write((ArgTag::Int, i)),
            Str(ref s) => w.write((ArgTag::Str, s)),
            List(ref l) => w.write((ArgTag::List, l)),
            Map(ref m) => w.write((ArgTag::Map, m)),
        }
    }

    fn size(&self) -> usize {
        use self::ExtraArg::*;
        let inner_size = match *self {
            Int(i) => i.size(),
            Str(ref s) => s.size(),
            List(ref l) => l.size(),
            Map(ref m) => m.size(),
        };
        1 + inner_size
    }

    fn size_is_fixed() -> bool { false }
}
