use std::collections::HashMap;

use types::*;

use input::InputBits;

pub use super::World;
pub use super::{Client, Entity, Inventory, Plane, TerrainChunk, Structure};


#[derive(Copy, PartialEq, Eq, Debug)]
pub enum EntityAttachment {
    World,
    Chunk,
    Client(ClientId),
}

#[derive(Copy, PartialEq, Eq, Debug)]
pub enum StructureAttachment {
    Plane,
    Chunk,
}

#[derive(Copy, PartialEq, Eq, Debug)]
pub enum InventoryAttachment {
    World,
    Client(ClientId),
    Entity(EntityId),
    Structure(StructureId),
}


impl super::Client {
    pub fn name(&self) -> &str {
        &*self.name
    }

    pub fn pawn_id(&self) -> Option<EntityId> {
        self.pawn
    }

    pub fn current_input(&self) -> InputBits {
        self.current_input
    }

    pub fn set_current_input(&mut self, new: InputBits) {
        self.current_input = new;
    }

    #[deprecated]
    pub fn chunk_offset(&self) -> (u8, u8) {
        (0, 0)
    }
}

impl super::Entity {
    pub fn plane_id(&self) -> PlaneId {
        self.plane
    }

    pub fn stable_plane_id(&self) -> Stable<PlaneId> {
        self.stable_plane
    }

    pub fn motion(&self) -> &Motion {
        &self.motion
    }

    // No motion_mut since modifying `self.motion` affects lookup tables.

    pub fn anim(&self) -> AnimId {
        self.anim
    }

    pub fn set_anim(&mut self, new: AnimId) {
        self.anim = new;
    }

    pub fn facing(&self) -> V3 {
        self.facing
    }

    pub fn set_facing(&mut self, new: V3) {
        self.facing = new;
    }

    pub fn target_velocity(&self) -> V3 {
        self.target_velocity
    }

    pub fn set_target_velocity(&mut self, new: V3) {
        self.target_velocity = new;
    }

    pub fn appearance(&self) -> u32 {
        self.appearance
    }

    pub fn pos(&self, now: Time) -> V3 {
        self.motion.pos(now)
    }

    pub fn attachment(&self) -> EntityAttachment {
        self.attachment
    }
}

impl super::Inventory {
    pub fn count(&self, item_id: ItemId) -> u8 {
        self.contents.get(&item_id).map_or(0, |&x| x)
    }

    pub fn contents(&self) -> &HashMap<ItemId, u8> {
        &self.contents
    }

    pub fn attachment(&self) -> InventoryAttachment {
        self.attachment
    }
}

impl super::Plane {
}

impl super::TerrainChunk {
    pub fn plane_id(&self) -> PlaneId {
        self.plane
    }

    pub fn chunk_pos(&self) -> V2 {
        self.cpos
    }

    pub fn block(&self, idx: usize) -> BlockId {
        self.blocks[idx]
    }

    pub fn blocks(&self) -> &BlockChunk {
        &*self.blocks
    }
}

impl super::Structure {
    pub fn pos(&self) -> V3 {
        self.pos
    }

    pub fn template_id(&self) -> TemplateId {
        self.template
    }

    pub fn attachment(&self) -> StructureAttachment {
        self.attachment
    }
}


// TODO: find somewhere better to put Motion

#[derive(Clone, Debug)]
pub struct Motion {
    pub start_time: Time,
    pub duration: Duration,
    pub start_pos: V3,
    pub end_pos: V3,
}

impl Motion {
    pub fn fixed(pos: V3) -> Motion {
        Motion {
            start_time: 0,
            duration: 0,
            start_pos: pos,
            end_pos: pos,
        }
    }

    pub fn stationary(pos: V3, now: Time) -> Motion {
        Motion {
            start_time: now,
            duration: -1 as Duration,
            start_pos: pos,
            end_pos: pos,
        }
    }

    pub fn pos(&self, now: Time) -> V3 {
        if now <= self.start_time {
            self.start_pos
        } else {
            let delta = now - self.start_time;
            if delta >= self.duration as Time {
                self.end_pos
            } else {
                let offset = (self.end_pos - self.start_pos) *
                        scalar(delta as i32) / scalar(self.duration as i32);
                self.start_pos + offset
            }
        }
    }

    pub fn end_time(&self) -> Time {
        self.start_time + self.duration as Time
    }
}
