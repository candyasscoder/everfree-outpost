use std::collections::hash_set;
use std::ops::{Deref, DerefMut};

use physics::CHUNK_SIZE;
use physics::Shape;

use data::ObjectTemplate;
use types::*;
use world::{World, Update};
use world::{Client, TerrainChunk, Entity, Structure, Inventory};
use world::{EntitiesById, StructuresById, InventoriesById};
use super::{EntityAttachment, StructureAttachment, InventoryAttachment};
use world::Motion;
use world::hooks::{Hooks, NoHooks};
use world::ops::{self, OpResult};
use util::Stable;


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
// TODO: should really be able to just derive Copy, but it tries O: Copy instead of O::Id: Copy
// TODO: turns out you can write this copy impl, but actually using it zeroes out the original
// (memory corruption, null &s)
//impl<'a, 'd, O: Object> Copy for ObjectRef<'a, 'd, O> { }

pub struct ObjectRefMut<'a, 'd: 'a, O: Object, H: Hooks+'a = NoHooks> {
    pub world: &'a mut World<'d>,
    pub hooks: &'a mut H,
    pub id: <O as Object>::Id,
}

pub trait ObjectRefBase<'d, O: Object> {
    fn world(&self) -> &World<'d>;
    fn id(&self) -> <O as Object>::Id;
    fn obj(&self) -> &O;
}

pub trait ObjectRefMutBase<'d, O: Object, H: Hooks>: ObjectRefBase<'d, O> {
    fn world_mut(&mut self) -> &mut World<'d>;
    fn hooks_mut(&mut self) -> &mut H;
    fn wh_mut(&mut self) -> (&mut World<'d>, &mut H);
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

impl<'a, 'd, O: Object, H: Hooks> ObjectRefBase<'d, O> for ObjectRefMut<'a, 'd, O, H> {
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

impl<'a, 'd, O: Object, H: Hooks> ObjectRefMutBase<'d, O, H> for ObjectRefMut<'a, 'd, O, H> {
    fn world_mut<'b>(&'b mut self) -> &'b mut World<'d> {
        &mut *self.world
    }

    fn hooks_mut(&mut self) -> &mut H {
        &mut *self.hooks
    }

    fn wh_mut(&mut self) -> (&mut World<'d>, &mut H) {
        (&mut *self.world, &mut *self.hooks)
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

impl<'a, 'd, O: Object, H: Hooks> Deref for ObjectRefMut<'a, 'd, O, H> {
    type Target = O;
    fn deref<'b>(&'b self) -> &'b O {
        self.obj()
    }
}

impl<'a, 'd, O: Object, H: Hooks> DerefMut for ObjectRefMut<'a, 'd, O, H> {
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

    fn child_entities<'b>(&'b self)
            -> EntitiesById<'b, 'd, hash_set::Iter<'b, EntityId>> {
        EntitiesById {
            world: self.world(),
            iter: self.obj().child_entities.iter(),
        }
    }

    fn child_inventories<'b>(&'b self)
            -> InventoriesById<'b, 'd, hash_set::Iter<'b, InventoryId>> {
        InventoriesById {
            world: self.world(),
            iter: self.obj().child_inventories.iter(),
        }
    }
}
impl<'a, 'd> ClientRef<'d> for ObjectRef<'a, 'd, Client> { }
impl<'a, 'd, H: Hooks> ClientRef<'d> for ObjectRefMut<'a, 'd, Client, H> { }

pub trait ClientRefMut<'d, H: Hooks>: ObjectRefMutBase<'d, Client, H> {
    fn stable_id(&mut self) -> Stable<ClientId> {
        let cid = self.id();
        self.world_mut().clients.pin(cid)
    }

    fn pawn_mut<'b>(&'b mut self) -> Option<ObjectRefMut<'b, 'd, Entity, H>> {
        match self.obj().pawn {
            None => None,
            Some(eid) => {
                let (w,h) = self.wh_mut();
                Some(w.entity_mut_hooks(h, eid))
            },
        }
    }

    fn set_pawn(&mut self, pawn: Option<EntityId>) -> OpResult<Option<EntityId>> {
        let cid = self.id();
        let (w,h) = self.wh_mut();
        match pawn {
            Some(eid) => ops::client_set_pawn(w, h, cid, eid),
            None => ops::client_clear_pawn(w, h, cid),
        }
    }
}
impl<'a, 'd, H: Hooks> ClientRefMut<'d, H> for ObjectRefMut<'a, 'd, Client, H> { }


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

    fn child_structures<'b>(&'b self)
            -> StructuresById<'b, 'd, hash_set::Iter<'b, StructureId>> {
        StructuresById {
            world: self.world(),
            iter: self.obj().child_structures.iter(),
        }
    }
}
impl<'a, 'd> TerrainChunkRef<'d> for ObjectRef<'a, 'd, TerrainChunk> { }
impl<'a, 'd, H: Hooks> TerrainChunkRef<'d> for ObjectRefMut<'a, 'd, TerrainChunk, H> { }

pub trait TerrainChunkRefMut<'d, H: Hooks>: ObjectRefMutBase<'d, TerrainChunk, H> {
}
impl<'a, 'd, H: Hooks> TerrainChunkRefMut<'d, H> for ObjectRefMut<'a, 'd, TerrainChunk, H> { }

fn block_pos_to_idx<'d, R: ?Sized+TerrainChunkRef<'d>>(self_: &R, pos: V3) -> usize {
    let offset = pos - self_.base_pos();
    Region::new(scalar(0), scalar(CHUNK_SIZE)).index(offset)
}


pub trait EntityRef<'d>: ObjectRefBase<'d, Entity> {
    fn child_inventories<'b>(&'b self)
            -> InventoriesById<'b, 'd, hash_set::Iter<'b, InventoryId>> {
        InventoriesById {
            world: self.world(),
            iter: self.obj().child_inventories.iter(),
        }
    }
}
impl<'a, 'd> EntityRef<'d> for ObjectRef<'a, 'd, Entity> { }
impl<'a, 'd, H: Hooks> EntityRef<'d> for ObjectRefMut<'a, 'd, Entity, H> { }

pub trait EntityRefMut<'d, H: Hooks>: ObjectRefMutBase<'d, Entity, H> {
    fn stable_id(&mut self) -> Stable<EntityId> {
        let eid = self.id();
        self.world_mut().entities.pin(eid)
    }

    fn set_motion(&mut self, motion: Motion) {
        let eid = self.id();
        // TODO: update entity-by-chunk cache
        self.obj_mut().motion = motion;
        self.world_mut().record(Update::EntityMotionChange(eid));
    }

    fn set_attachment(&mut self, attach: EntityAttachment) -> OpResult<EntityAttachment> {
        let eid = self.id();
        let (w,h) = self.wh_mut();
        ops::entity_attach(w, h, eid, attach)
    }
}
impl<'a, 'd, H: Hooks> EntityRefMut<'d, H> for ObjectRefMut<'a, 'd, Entity, H> { }


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

    fn child_inventories<'b>(&'b self)
            -> InventoriesById<'b, 'd, hash_set::Iter<'b, InventoryId>> {
        InventoriesById {
            world: self.world(),
            iter: self.obj().child_inventories.iter(),
        }
    }
}
impl<'a, 'd> StructureRef<'d> for ObjectRef<'a, 'd, Structure> { }
impl<'a, 'd, H: Hooks> StructureRef<'d> for ObjectRefMut<'a, 'd, Structure, H> { }

pub trait StructureRefMut<'d, H: Hooks>: ObjectRefMutBase<'d, Structure, H> {
    fn stable_id(&mut self) -> Stable<StructureId> {
        let sid = self.id();
        self.world_mut().structures.pin(sid)
    }

    fn set_pos(&mut self, pos: V3) -> OpResult<()> {
        let sid = self.id();
        let (w,h) = self.wh_mut();
        ops::structure_move(w, h, sid, pos)
    }

    fn set_template_id(&mut self, template: TemplateId) -> OpResult<()> {
        let sid = self.id();
        let (w,h) = self.wh_mut();
        ops::structure_replace(w, h, sid, template)
    }

    fn set_attachment(&mut self, attach: StructureAttachment) -> OpResult<StructureAttachment> {
        let sid = self.id();
        let (w,h) = self.wh_mut();
        ops::structure_attach(w, h, sid, attach)
    }
}
impl<'a, 'd, H: Hooks> StructureRefMut<'d, H> for ObjectRefMut<'a, 'd, Structure, H> { }


pub trait InventoryRef<'d>: ObjectRefBase<'d, Inventory> {
    fn count_by_name(&self, name: &str) -> OpResult<u8> {
        let item_id = unwrap!(self.world().data().item_data.find_id(name));
        Ok(self.obj().count(item_id))
    }
}
impl<'a, 'd> InventoryRef<'d> for ObjectRef<'a, 'd, Inventory> { }
impl<'a, 'd, H: Hooks> InventoryRef<'d> for ObjectRefMut<'a, 'd, Inventory, H> { }

pub trait InventoryRefMut<'d, H: Hooks>: ObjectRefMutBase<'d, Inventory, H> {
    fn stable_id(&mut self) -> Stable<InventoryId> {
        let iid = self.id();
        self.world_mut().inventories.pin(iid)
    }

    fn update(&mut self, item_id: ItemId, adjust: i16) -> u8 {
        let iid = self.id();
        let (w,h) = self.wh_mut();
        // OK: self.id() is always a valid InventoryId
        ops::inventory_update(w, h, iid, item_id, adjust).unwrap()
    }

    fn update_by_name(&mut self, name: &str, adjust: i16) -> OpResult<u8> {
        let item_id = unwrap!(self.world().data().item_data.find_id(name));
        Ok(self.update(item_id, adjust))
    }

    fn set_attachment(&mut self, attach: InventoryAttachment) -> OpResult<InventoryAttachment> {
        let iid = self.id();
        let (w,h) = self.wh_mut();
        ops::inventory_attach(w, h, iid, attach)
    }
}
impl<'a, 'd, H: Hooks> InventoryRefMut<'d, H> for ObjectRefMut<'a, 'd, Inventory, H> { }
