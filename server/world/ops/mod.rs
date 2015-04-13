use util::StrResult;

pub type OpResult<T> = StrResult<T>;

pub mod client;
pub mod terrain_chunk;
pub mod entity;
pub mod structure;
pub mod inventory;
