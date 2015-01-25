use std::mem::replace;
use std::ops::{Deref, DerefMut};
use std::ptr;

use physics::v3::{Vn, V3, V2};

use data::ObjectTemplate;
use input::InputBits;
use types::*;
use view::ViewState;
use world::World;
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
}

pub struct ObjectRefMut<'a, 'd: 'a, O: Object> {
    pub world: &'a mut World<'d>,
    pub id: <O as Object>::Id,
}

impl<'a, 'd, O: Object> ObjectRef<'a, 'd, O> {
    fn world(&self) -> &'a World<'d> {
        &*self.world
    }

    fn id(&self) -> <O as Object>::Id {
        self.id
    }

    fn obj(&self) -> &'a O {
        <O as Object>::get(self.world, self.id)
            .expect("tried to call ObjectRef::obj() after deleting the object")
    }
}

impl<'a, 'd, O: Object> ObjectRefMut<'a, 'd, O> {
    fn downgrade<'b>(&'b self) -> ObjectRef<'b, 'd, O> {
        ObjectRef {
            world: self.world,
            id: self.id,
        }
    }

    fn world(&self) -> &World<'d> { self.downgrade().world() }
    fn world_mut<'b>(&'b mut self) -> &'b mut World<'d> {
        &mut *self.world
    }

    fn id(&self) -> <O as Object>::Id { self.downgrade().id() }

    fn obj(&self) -> &O { self.downgrade().obj() }
    fn obj_mut<'b>(&'b mut self) -> &'b mut O {
        <O as Object>::get_mut(self.world, self.id)
            .expect("tried to call ObjectRef::obj_mut() after deleting the object")
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


impl<'a, 'd> ObjectRef<'a, 'd, Client> {
    fn pawn(&self) -> Option<ObjectRef<'a, 'd, Entity>> {
        match self.obj().pawn {
            None => None,
            Some(eid) => Some(self.world().entity(eid)),
        }
    }
}

impl<'a, 'd> ObjectRefMut<'a, 'd, Client> {
    fn pawn<'b>(&'b self) -> Option<ObjectRef<'b, 'd, Entity>> { self.downgrade().pawn() }
    fn pawn_mut<'b>(&'b mut self) -> Option<ObjectRefMut<'b, 'd, Entity>> {
        match self.obj().pawn {
            None => None,
            Some(eid) => Some(self.world_mut().entity_mut(eid)),
        }
    }

    fn set_pawn(&mut self, now: Time, pawn: Option<EntityId>) -> OpResult<Option<EntityId>> {
        match pawn {
            Some(eid) => ops::client_set_pawn(self.world, now, self.id, eid),
            None => ops::client_clear_pawn(self.world, self.id),
        }
    }
}

impl<'a, 'd> ObjectRefMut<'a, 'd, Entity> {
    fn set_motion(&mut self, motion: Motion) -> OpResult<Motion> {
        unimplemented!()
    }

    fn set_attachment(&mut self, attach: EntityAttachment) -> OpResult<EntityAttachment> {
        ops::entity_attach(self.world, self.id, attach)
    }
}

impl<'a, 'd> ObjectRef<'a, 'd, Structure> {
    fn template(&self) -> &'d ObjectTemplate {
        let tid = self.template_id();
        self.world.data.object_templates.template(self.template_id())
    }
}

impl<'a, 'd> ObjectRefMut<'a, 'd, Structure> {
    fn template(&self) -> &'d ObjectTemplate {
        let tid = self.template_id();
        self.world.data.object_templates.template(self.template_id())
    }

    fn set_pos(&mut self, pos: V3) -> OpResult<()> {
        ops::structure_move(self.world, self.id, pos)
    }

    fn set_template_id(&mut self, template: TemplateId) -> OpResult<()> {
        ops::structure_replace(self.world, self.id, template)
    }
}
