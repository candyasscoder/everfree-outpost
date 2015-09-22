use libserver_types::V2;

pub use self::provider::Provider;

mod summary;
mod provider;

mod plan;
mod caves;
mod vault;

const DUNGEON_SIZE: i32 = 256;
const ENTRANCE_POS: V2 = V2 { x: DUNGEON_SIZE / 2, y: DUNGEON_SIZE / 2 };
