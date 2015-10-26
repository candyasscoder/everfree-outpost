use types::*;

pub use self::error::{Error, Result};
pub use self::writer::Writer;
pub use self::reader::Reader;
pub use self::object_writer::{ObjectWriter, WriteHooks};
pub use self::object_reader::{ObjectReader, ReadHooks};
pub use self::object_reader::Fragment as ReadFragment;

mod error;
// TODO: these shouldn't need to be public, but otherwise rustc complains that "source trait is
// inaccessible".
pub mod writer;
pub mod reader;
mod object_writer;
mod object_reader;


type SaveId = u32;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum AnyId {
    Client(ClientId),
    Entity(EntityId),
    Inventory(InventoryId),
    Plane(PlaneId),
    TerrainChunk(TerrainChunkId),
    Structure(StructureId),
}


pub trait ToAnyId {
    fn to_any_id(self) -> AnyId;
}

impl ToAnyId for AnyId {
    fn to_any_id(self) -> AnyId { self }
}

impl ToAnyId for ClientId {
    fn to_any_id(self) -> AnyId { AnyId::Client(self) }
}

impl ToAnyId for EntityId {
    fn to_any_id(self) -> AnyId { AnyId::Entity(self) }
}

impl ToAnyId for InventoryId {
    fn to_any_id(self) -> AnyId { AnyId::Inventory(self) }
}

impl ToAnyId for PlaneId {
    fn to_any_id(self) -> AnyId { AnyId::Plane(self) }
}

impl ToAnyId for TerrainChunkId {
    fn to_any_id(self) -> AnyId { AnyId::TerrainChunk(self) }
}

impl ToAnyId for StructureId {
    fn to_any_id(self) -> AnyId { AnyId::Structure(self) }
}


const CURRENT_VERSION: u32 = 6;


fn padding(len: usize) -> usize {
    (4 - (len % 4)) % 4
}
