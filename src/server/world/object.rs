use std::collections::hash_set;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

use libphysics::CHUNK_SIZE;
use libphysics::Shape;

use data::StructureTemplate;
use types::*;
use world::World;
use world::{Client, Entity, Inventory, Plane, TerrainChunk, Structure};
use world::{EntitiesById, StructuresById, InventoriesById};
use world::{EntityAttachment, StructureAttachment, InventoryAttachment};
use world::{TerrainChunkFlags, StructureFlags};
use world::Motion;
use world::fragment::Fragment;
use world::hooks::Hooks;
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

impl Object for Entity {
    type Id = EntityId;

    fn get<'a>(world: &'a World, id: EntityId) -> Option<&'a Entity> {
        world.entities.get(id)
    }

    fn get_mut<'a>(world: &'a mut World, id: EntityId) -> Option<&'a mut Entity> {
        world.entities.get_mut(id)
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

impl Object for Plane {
    type Id = PlaneId;

    fn get<'a>(world: &'a World, id: PlaneId) -> Option<&'a Plane> {
        world.planes.get(id)
    }

    fn get_mut<'a>(world: &'a mut World, id: PlaneId) -> Option<&'a mut Plane> {
        world.planes.get_mut(id)
    }
}

impl Object for TerrainChunk {
    type Id = TerrainChunkId;

    fn get<'a>(world: &'a World, id: TerrainChunkId) -> Option<&'a TerrainChunk> {
        world.terrain_chunks.get(id)
    }

    fn get_mut<'a>(world: &'a mut World, id: TerrainChunkId) -> Option<&'a mut TerrainChunk> {
        world.terrain_chunks.get_mut(id)
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


pub struct ObjectRef<'a, 'd: 'a, O: Object> {
    pub world: &'a World<'d>,
    pub id: <O as Object>::Id,
    pub obj: &'a O,
}
// TODO: should really be able to just derive Copy, but it tries O: Copy instead of O::Id: Copy
// TODO: turns out you can write this copy impl, but actually using it zeroes out the original
// (memory corruption, null &s)
//impl<'a, 'd, O: Object> Copy for ObjectRef<'a, 'd, O> { }

pub struct ObjectRefMut<'a, 'd, O: Object, F: Fragment<'d>+'a> {
    pub fragment: &'a mut F,
    pub id: <O as Object>::Id,
    _marker0: PhantomData<&'d ()>,
}

impl<'a, 'd, O: Object, F: Fragment<'d>> ObjectRefMut<'a, 'd, O, F> {
    pub fn new(fragment: &'a mut F, id: <O as Object>::Id) -> ObjectRefMut<'a, 'd, O, F> {
        ObjectRefMut {
            fragment: fragment,
            id: id,
            _marker0: PhantomData,
        }
    }
}

pub trait ObjectRefBase<'d, O: Object> {
    fn world(&self) -> &World<'d>;
    fn id(&self) -> <O as Object>::Id;
    fn obj(&self) -> &O;
}

pub trait ObjectRefMutBase<'d, O: Object, F: Fragment<'d>>: ObjectRefBase<'d, O> {
    fn world_mut(&mut self) -> &mut World<'d>;
    fn fragment_mut(&mut self) -> &mut F;
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

impl<'a, 'd, O: Object, F: Fragment<'d>> ObjectRefBase<'d, O> for ObjectRefMut<'a, 'd, O, F> {
    fn world(&self) -> &World<'d> {
        self.fragment.world()
    }

    fn id(&self) -> <O as Object>::Id {
        self.id
    }

    fn obj(&self) -> &O {
        <O as Object>::get(self.world(), self.id)
            .expect("tried to call ObjectRefMut::obj() after deleting the object")
    }
}

impl<'a, 'd, O: Object, F: Fragment<'d>> ObjectRefMutBase<'d, O, F> for ObjectRefMut<'a, 'd, O, F> {
    fn world_mut(&mut self) -> &mut World<'d> {
        self.fragment.world_mut()
    }

    fn fragment_mut(&mut self) -> &mut F {
        self.fragment
    }

    fn obj_mut(&mut self) -> &mut O {
        <O as Object>::get_mut(self.fragment.world_mut(), self.id)
            .expect("tried to call ObjectRefMut::obj_mut() after deleting the object")
    }
}

impl<'a, 'd, O: Object> Deref for ObjectRef<'a, 'd, O> {
    type Target = O;
    fn deref(&self) -> &O {
        self.obj()
    }
}

impl<'a, 'd, O: Object, F: Fragment<'d>> Deref for ObjectRefMut<'a, 'd, O, F> {
    type Target = O;
    fn deref(&self) -> &O {
        self.obj()
    }
}

impl<'a, 'd, O: Object, F: Fragment<'d>> DerefMut for ObjectRefMut<'a, 'd, O, F> {
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
        EntitiesById::new(self.world(), self.obj().child_entities.iter())
    }

    fn child_inventories<'b>(&'b self)
            -> InventoriesById<'b, 'd, hash_set::Iter<'b, InventoryId>> {
        InventoriesById::new(self.world(), self.obj().child_inventories.iter())
    }
}
impl<'a, 'd> ClientRef<'d> for ObjectRef<'a, 'd, Client> { }
impl<'a, 'd, F: Fragment<'d>> ClientRef<'d> for ObjectRefMut<'a, 'd, Client, F> { }

pub trait ClientRefMut<'d, F: Fragment<'d>>: ObjectRefMutBase<'d, Client, F> {
    fn stable_id(&mut self) -> Stable<ClientId> {
        let cid = self.id();
        self.world_mut().clients.pin(cid)
    }

    fn pawn_mut<'a>(&'a mut self) -> Option<ObjectRefMut<'a, 'd, Entity, F>> {
        match self.obj().pawn {
            None => None,
            Some(eid) => {
                Some(self.fragment_mut().entity_mut(eid))
            },
        }
    }

    fn set_pawn(&mut self, pawn: Option<EntityId>) -> OpResult<Option<EntityId>> {
        let cid = self.id();
        match pawn {
            Some(eid) => ops::client::set_pawn(self.fragment_mut(), cid, eid),
            None => ops::client::clear_pawn(self.fragment_mut(), cid),
        }
    }
}
impl<'a, 'd, F: Fragment<'d>> ClientRefMut<'d, F> for ObjectRefMut<'a, 'd, Client, F> { }


pub trait EntityRef<'d>: ObjectRefBase<'d, Entity> {
    fn child_inventories<'b>(&'b self)
            -> InventoriesById<'b, 'd, hash_set::Iter<'b, InventoryId>> {
        InventoriesById::new(self.world(), self.obj().child_inventories.iter())
    }

    fn pawn_owner<'b>(&'b self) -> Option<ObjectRef<'b, 'd, Client>> {
        let eid = self.id();
        if let EntityAttachment::Client(cid) = self.obj().attachment {
            let c = unwrap_or!(self.world().get_client(cid), return None);
            if c.pawn_id() == Some(eid) {
                return Some(c)
            }
        }
        None
    }
}
impl<'a, 'd> EntityRef<'d> for ObjectRef<'a, 'd, Entity> { }
impl<'a, 'd, F: Fragment<'d>> EntityRef<'d> for ObjectRefMut<'a, 'd, Entity, F> { }

pub trait EntityRefMut<'d, F: Fragment<'d>>: ObjectRefMutBase<'d, Entity, F> {
    fn stable_id(&mut self) -> Stable<EntityId> {
        let eid = self.id();
        self.world_mut().entities.pin(eid)
    }

    fn set_plane_id(&mut self, pid: PlaneId) -> OpResult<()> {
        let eid = self.id();
        ops::entity::set_plane(self.fragment_mut(), eid, pid)
    }

    fn set_stable_plane_id(&mut self, stable_pid: Stable<PlaneId>) -> OpResult<()> {
        let eid = self.id();
        ops::entity::set_stable_plane(self.fragment_mut(), eid, stable_pid)
    }

    fn set_motion(&mut self, motion: Motion) {
        let eid = self.id();
        // TODO: update entity-by-chunk cache
        self.obj_mut().motion = motion;
        self.fragment_mut().with_hooks(|h| h.on_entity_motion_change(eid));
    }

    fn set_appearance(&mut self, appearance: u32) {
        let eid = self.id();
        self.obj_mut().appearance = appearance;
        self.fragment_mut().with_hooks(|h| h.on_entity_appearance_change(eid));
    }

    fn set_attachment(&mut self, attach: EntityAttachment) -> OpResult<EntityAttachment> {
        let eid = self.id();
        ops::entity::attach(self.fragment_mut(), eid, attach)
    }

    fn pawn_owner_mut<'a>(&'a mut self) -> Option<ObjectRefMut<'a, 'd, Client, F>> {
        let eid = self.id();
        if let EntityAttachment::Client(cid) = self.obj().attachment {
            let c = unwrap_or!(self.fragment_mut().get_client_mut(cid), return None);
            if c.pawn_id() == Some(eid) {
                return Some(c)
            }
        }
        None
    }
}
impl<'a, 'd, F: Fragment<'d>> EntityRefMut<'d, F> for ObjectRefMut<'a, 'd, Entity, F> { }



pub trait InventoryRef<'d>: ObjectRefBase<'d, Inventory> {
    fn count_by_name(&self, name: &str) -> OpResult<u8> {
        let item_id = unwrap!(self.world().data().item_data.find_id(name));
        Ok(self.obj().count(item_id))
    }
}
impl<'a, 'd> InventoryRef<'d> for ObjectRef<'a, 'd, Inventory> { }
impl<'a, 'd, F: Fragment<'d>> InventoryRef<'d> for ObjectRefMut<'a, 'd, Inventory, F> { }

pub trait InventoryRefMut<'d, F: Fragment<'d>>: ObjectRefMutBase<'d, Inventory, F> {
    fn stable_id(&mut self) -> Stable<InventoryId> {
        let iid = self.id();
        self.world_mut().inventories.pin(iid)
    }

    fn update(&mut self, item_id: ItemId, adjust: i16) -> u8 {
        let iid = self.id();
        // OK: self.id() is always a valid InventoryId
        ops::inventory::update(self.fragment_mut(), iid, item_id, adjust).unwrap()
    }

    fn update_by_name(&mut self, name: &str, adjust: i16) -> OpResult<u8> {
        let item_id = unwrap!(self.world().data().item_data.find_id(name));
        Ok(self.update(item_id, adjust))
    }

    fn set_attachment(&mut self, attach: InventoryAttachment) -> OpResult<InventoryAttachment> {
        let iid = self.id();
        ops::inventory::attach(self.fragment_mut(), iid, attach)
    }
}
impl<'a, 'd, F: Fragment<'d>> InventoryRefMut<'d, F> for ObjectRefMut<'a, 'd, Inventory, F> { }



pub trait PlaneRef<'d>: ObjectRefBase<'d, Plane> {
    fn get_terrain_chunk<'b>(&'b self, cpos: V2) -> Option<ObjectRef<'b, 'd, TerrainChunk>> {
        let &tcid = unwrap_or!(self.obj().loaded_chunks.get(&cpos), return None);
        // Since `tcid` is in `loaded_chunks`, there should exist a chunk with that ID.
        Some(self.world().terrain_chunk(tcid))
    }

    fn terrain_chunk<'b>(&'b self, cpos: V2) -> ObjectRef<'b, 'd, TerrainChunk> {
        self.get_terrain_chunk(cpos).expect("no TerrainChunk at given pos")
    }
}
impl<'a, 'd> PlaneRef<'d> for ObjectRef<'a, 'd, Plane> { }
impl<'a, 'd, F: Fragment<'d>> PlaneRef<'d> for ObjectRefMut<'a, 'd, Plane, F> { }

pub trait PlaneRefMut<'d, F: Fragment<'d>>: ObjectRefMutBase<'d, Plane, F> {
    fn stable_id(&mut self) -> Stable<PlaneId> {
        let pid = self.id();
        self.world_mut().planes.pin(pid)
    }

    fn get_terrain_chunk_mut<'b>(&'b mut self, cpos: V2)
                                 -> Option<ObjectRefMut<'b, 'd, TerrainChunk, F>> {
        let &tcid = unwrap_or!(self.obj().loaded_chunks.get(&cpos), return None);
        Some(self.fragment_mut().terrain_chunk_mut(tcid))
    }

    fn terrain_chunk_mut<'b>(&'b mut self, cpos: V2) -> ObjectRefMut<'b, 'd, TerrainChunk, F> {
        self.get_terrain_chunk_mut(cpos).expect("no TerrainChunk at given pos")
    }

    fn try_save_terrain_chunk(&mut self, cpos: V2) -> Option<Stable<TerrainChunkId>> {
        let result = self.get_terrain_chunk_mut(cpos).map(|mut tc| tc.stable_id());
        if let Some(stable_tcid) = result {
            self.obj_mut().saved_chunks.insert(cpos, stable_tcid);
        }
        result
    }

    fn save_terrain_chunk(&mut self, cpos: V2) -> Stable<TerrainChunkId> {
        self.try_save_terrain_chunk(cpos).expect("no TerrainChunk at given pos")
    }
}
impl<'a, 'd, F: Fragment<'d>> PlaneRefMut<'d, F> for ObjectRefMut<'a, 'd, Plane, F> { }



pub trait TerrainChunkRef<'d>: ObjectRefBase<'d, TerrainChunk> {
    fn base_pos(&self) -> V3 {
        self.obj().chunk_pos().extend(0) * scalar(CHUNK_SIZE)
    }

    fn bounds(&self) -> Region {
        let pos = self.base_pos();
        Region::new(pos, pos + scalar(CHUNK_SIZE))
    }

    fn blocks(&self) -> &BlockChunk {
        &*self.obj().blocks
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

    fn flags(&self) -> TerrainChunkFlags {
        self.obj().flags
    }

    fn child_structures<'b>(&'b self)
            -> StructuresById<'b, 'd, hash_set::Iter<'b, StructureId>> {
        StructuresById::new(self.world(), self.obj().child_structures.iter())
    }
}
impl<'a, 'd> TerrainChunkRef<'d> for ObjectRef<'a, 'd, TerrainChunk> { }
impl<'a, 'd, F: Fragment<'d>> TerrainChunkRef<'d> for ObjectRefMut<'a, 'd, TerrainChunk, F> { }

pub trait TerrainChunkRefMut<'d, F: Fragment<'d>>: ObjectRefMutBase<'d, TerrainChunk, F> {
    fn stable_id(&mut self) -> Stable<TerrainChunkId> {
        let tcid = self.id();
        self.world_mut().terrain_chunks.pin(tcid)
    }

    fn blocks_mut(&mut self) -> &mut BlockChunk {
        &mut *self.obj_mut().blocks
    }

    fn flags_mut(&mut self) -> &mut TerrainChunkFlags {
        &mut self.obj_mut().flags
    }
}
impl<'a, 'd, F: Fragment<'d>> TerrainChunkRefMut<'d, F> for ObjectRefMut<'a, 'd, TerrainChunk, F> { }

fn block_pos_to_idx<'d, R: ?Sized+TerrainChunkRef<'d>>(self_: &R, pos: V3) -> usize {
    let offset = pos - self_.base_pos();
    Region::new(scalar(0), scalar(CHUNK_SIZE)).index(offset)
}



pub trait StructureRef<'d>: ObjectRefBase<'d, Structure> {
    fn template(&self) -> &'d StructureTemplate {
        self.world().data.structure_templates.template(self.obj().template_id())
    }

    fn size(&self) -> V3 {
        self.template().size
    }

    fn bounds(&self) -> Region {
        let pos = self.obj().pos();
        let size = self.size();
        Region::new(pos, pos + size)
    }

    fn flags(&self) -> StructureFlags {
        self.obj().flags
    }

    fn child_inventories<'b>(&'b self)
            -> InventoriesById<'b, 'd, hash_set::Iter<'b, InventoryId>> {
        InventoriesById::new(self.world(), self.obj().child_inventories.iter())
    }
}
impl<'a, 'd> StructureRef<'d> for ObjectRef<'a, 'd, Structure> { }
impl<'a, 'd, F: Fragment<'d>> StructureRef<'d> for ObjectRefMut<'a, 'd, Structure, F> { }

pub trait StructureRefMut<'d, F: Fragment<'d>>: ObjectRefMutBase<'d, Structure, F> {
    fn stable_id(&mut self) -> Stable<StructureId> {
        let sid = self.id();
        self.world_mut().structures.pin(sid)
    }

    fn set_template_id(&mut self, template: TemplateId) -> OpResult<()> {
        let sid = self.id();
        ops::structure::replace(self.fragment_mut(), sid, template)
    }

    fn set_flags(&mut self, flags: StructureFlags) {
        self.obj_mut().flags = flags;
    }

    fn set_attachment(&mut self, attach: StructureAttachment) -> OpResult<StructureAttachment> {
        let sid = self.id();
        ops::structure::attach(self.fragment_mut(), sid, attach)
    }
}
impl<'a, 'd, F: Fragment<'d>> StructureRefMut<'d, F> for ObjectRefMut<'a, 'd, Structure, F> { }
