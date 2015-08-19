use util::StrResult;

pub type OpResult<T> = StrResult<T>;

pub mod client;
pub mod entity;
pub mod inventory;
pub mod plane;
pub mod terrain_chunk;
pub mod structure;
