use std::mem::replace;
use std::ops::{Deref, DerefMut};
use std::ptr;

use physics::CHUNK_SIZE;
use physics::Shape;
use physics::v3::{Vn, V3, V2, scalar, Region};

use data::ObjectTemplate;
use input::InputBits;
use types::*;
use view::ViewState;
use world::{World, Update};
use world::{Client, TerrainChunk, Entity, Structure, Inventory};
use super::{EntityAttachment, StructureAttachment, InventoryAttachment};
use world::Motion;
use world::ops::{self, OpResult};


pub trait Object: 'static {
    type Id: Copy;

    fn get<'a>(world: &'a World, id: <Self as Object>::Id) -> Option<&'a Self>;
    fn get_mut<'a>(world: &'a mut World, id: <Self as Object>::Id) -> Option<&'a mut Self>;
}

impl Object for Client {
    type Id = ClientId;

    fn get<'a>(world: &'a World, id: ClientId) -> Option<&'a Client> {
        world.clients.get(id)
    }

    fn get_mut<'a>(world: &'a mut World, id: ClientId) -> Option<&'a mut Client> {
        world.clients.get_mut(id)
    }
}

impl Object for TerrainChunk {
    type Id = V2;

    fn get<'a>(world: &'a World, id: V2) -> Option<&'a TerrainChunk> {
        world.terrain_chunks.get(&id)
    }

    fn get_mut<'a>(world: &'a mut World, id: V2) -> Option<&'a mut TerrainChunk> {
        world.terrain_chunks.get_mut(&id)
    }
}

impl Object for Entity {
    type Id = EntityId;

    fn get<'a>(world: &'a World, id: EntityId) -> Option<&'a Entity> {
        world.entities.get(id)
    }

    fn get_mut<'a>(world: &'a mut World, id: EntityId) -> Option<&'a mut Entity> {
        world.entities.get_mut(id)
    }
}

impl Object for Structure {
    type Id = StructureId;

    fn get<'a>(world: &'a World, id: StructureId) -> Option<&'a Structure> {
        world.structures.get(id)
    }

    fn get_mut<'a>(world: &'a mut World, id: StructureId) -> Option<&'a mut Structure> {
        world.structures.get_mut(id)
    }
}

impl Object for Inventory {
    type Id = InventoryId;

    fn get<'a>(world: &'a World, id: InventoryId) -> Option<&'a Inventory> {
        world.inventories.get(id)
    }

    fn get_mut<'a>(world: &'a mut World, id: InventoryId) -> Option<&'a mut Inventory> {
        world.inventories.get_mut(id)
    }
}


pub struct ObjectRef<'a, 'd: 'a, O: Object> {
    pub world: &'a World<'d>,
    pub id: <O as Object>::Id,
    pub obj: &'a O,
}
impl<'a, 'd, O: Object> Copy for ObjectRef<'a, 'd, O> { }

pub struct ObjectRefMut<'a, 'd: 'a, O: Object> {
    pub world: &'a mut World<'d>,
    pub id: <O as Object>::Id,
}

pub trait ObjectRefBase<'d, O: Object> {
    fn world(&self) -> &World<'d>;
    fn id(&self) -> <O as Object>::Id;
    fn obj(&self) -> &O;
}

pub trait ObjectRefMutBase<'d, O: Object>: ObjectRefBase<'d, O> {
    fn world_mut(&mut self) -> &mut World<'d>;
    fn obj_mut(&mut self) -> &mut O;
}

impl<'a, 'd, O: Object> ObjectRefBase<'d, O> for ObjectRef<'a, 'd, O> {
    fn world(&self) -> &World<'d> {
        self.world
    }

    fn id(&self) -> <O as Object>::Id {
        self.id
    }

    fn obj(&self) -> &O {
        self.obj
    }
}

impl<'a, 'd, O: Object> ObjectRefBase<'d, O> for ObjectRefMut<'a, 'd, O> {
    fn world(&self) -> &World<'d> {
        self.world
    }

    fn id(&self) -> <O as Object>::Id {
        self.id
    }

    fn obj(&self) -> &O {
        <O as Object>::get(self.world, self.id)
            .expect("tried to call ObjectRefMut::obj() after deleting the object")
    }
}

impl<'a, 'd, O: Object> ObjectRefMutBase<'d, O> for ObjectRefMut<'a, 'd, O> {
    fn world_mut<'b>(&'b mut self) -> &'b mut World<'d> {
        &mut *self.world
    }

    fn obj_mut<'b>(&'b mut self) -> &'b mut O {
        <O as Object>::get_mut(self.world, self.id)
            .expect("tried to call ObjectRefMut::obj_mut() after deleting the object")
    }
}

impl<'a, 'd, O: Object> Deref for ObjectRef<'a, 'd, O> {
    type Target = O;
    fn deref<'b>(&'b self) -> &'b O {
        self.obj()
    }
}

impl<'a, 'd, O: Object> Deref for ObjectRefMut<'a, 'd, O> {
    type Target = O;
    fn deref<'b>(&'b self) -> &'b O {
        self.obj()
    }
}

impl<'a, 'd, O: Object> DerefMut for ObjectRefMut<'a, 'd, O> {
    fn deref_mut<'b>(&'b mut self) -> &'b mut O {
        self.obj_mut()
    }
}


pub trait ClientRef<'d>: ObjectRefBase<'d, Client> {
    fn pawn<'b>(&'b self) -> Option<ObjectRef<'b, 'd, Entity>> {
        match self.obj().pawn {
            None => None,
            Some(eid) => Some(self.world().entity(eid)),
        }
    }

    fn camera_pos(&self, now: Time) -> V2 {
        self.pawn().map_or(scalar(0), |p| p.pos(now).reduce())
    }
}
impl<'a, 'd> ClientRef<'d> for ObjectRef<'a, 'd, Client> { }
impl<'a, 'd> ClientRef<'d> for ObjectRefMut<'a, 'd, Client> { }

pub trait ClientRefMut<'d>: ObjectRefMutBase<'d, Client> {
    fn pawn_mut<'b>(&'b mut self) -> Option<ObjectRefMut<'b, 'd, Entity>> {
        match self.obj().pawn {
            None => None,
            Some(eid) => Some(self.world_mut().entity_mut(eid)),
        }
    }

    fn set_pawn(&mut self, now: Time, pawn: Option<EntityId>) -> OpResult<Option<EntityId>> {
        let cid = self.id();
        match pawn {
            Some(eid) => ops::client_set_pawn(self.world_mut(), now, cid, eid),
            None => ops::client_clear_pawn(self.world_mut(), cid),
        }
    }
}
impl<'a, 'd> ClientRefMut<'d> for ObjectRefMut<'a, 'd, Client> { }

pub trait TerrainChunkRef<'d>: ObjectRefBase<'d, TerrainChunk> {
    fn base_pos(&self) -> V3 {
        self.id().extend(0) * scalar(CHUNK_SIZE)
    }

    fn bounds(&self) -> Region {
        let pos = self.base_pos();
        Region::new(pos, pos + scalar(CHUNK_SIZE))
    }

    fn block_at(&self, pos: V3) -> BlockId {
        self.obj().block(block_pos_to_idx(self, pos))
    }

    fn shape(&self, idx: usize) -> Shape {
        self.world().data.block_data.shape(self.obj().block(idx))
    }

    fn shape_at(&self, pos: V3) -> Shape {
        self.shape(block_pos_to_idx(self, pos))
    }
}
impl<'a, 'd> TerrainChunkRef<'d> for ObjectRef<'a, 'd, TerrainChunk> { }
impl<'a, 'd> TerrainChunkRef<'d> for ObjectRefMut<'a, 'd, TerrainChunk> { }

pub trait TerrainChunkRefMut<'d>: ObjectRefMutBase<'d, TerrainChunk> {
}
impl<'a, 'd> TerrainChunkRefMut<'d> for ObjectRefMut<'a, 'd, TerrainChunk> { }

fn block_pos_to_idx<'d, R: ?Sized+TerrainChunkRef<'d>>(self_: &R, pos: V3) -> usize {
    let offset = pos - self_.base_pos();
    Region::new(scalar(0), scalar(CHUNK_SIZE)).index(offset)
}

pub trait EntityRef<'d>: ObjectRefBase<'d, Entity> {
}
impl<'a, 'd> EntityRef<'d> for ObjectRef<'a, 'd, Entity> { }
impl<'a, 'd> EntityRef<'d> for ObjectRefMut<'a, 'd, Entity> { }

pub trait EntityRefMut<'d>: ObjectRefMutBase<'d, Entity> {
    fn set_motion(&mut self, motion: Motion) {
        let eid = self.id();
        // TODO: update entity-by-chunk cache
        self.obj_mut().motion = motion;
        self.world_mut().record(Update::EntityMotionChange(eid));
    }

    fn set_attachment(&mut self, attach: EntityAttachment) -> OpResult<EntityAttachment> {
        let eid = self.id();
        ops::entity_attach(self.world_mut(), eid, attach)
    }
}
impl<'a, 'd> EntityRefMut<'d> for ObjectRefMut<'a, 'd, Entity> { }

pub trait StructureRef<'d>: ObjectRefBase<'d, Structure> {
    fn template(&self) -> &'d ObjectTemplate {
        self.world().data.object_templates.template(self.obj().template_id())
    }

    fn size(&self) -> V3 {
        self.template().size
    }

    fn bounds(&self) -> Region {
        let pos = self.obj().pos();
        let size = self.size();
        Region::new(pos, pos + size)
    }
}
impl<'a, 'd> StructureRef<'d> for ObjectRef<'a, 'd, Structure> { }
impl<'a, 'd> StructureRef<'d> for ObjectRefMut<'a, 'd, Structure> { }

pub trait StructureRefMut<'d>: ObjectRefMutBase<'d, Structure> {
    fn set_pos(&mut self, pos: V3) -> OpResult<()> {
        let sid = self.id();
        ops::structure_move(self.world_mut(), sid, pos)
    }

    fn set_template_id(&mut self, template: TemplateId) -> OpResult<()> {
        let sid = self.id();
        ops::structure_replace(self.world_mut(), sid, template)
    }
}
impl<'a, 'd> StructureRefMut<'d> for ObjectRefMut<'a, 'd, Structure> { }
