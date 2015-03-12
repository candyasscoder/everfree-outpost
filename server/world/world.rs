use std::collections::{HashMap, hash_map, HashSet, hash_set};
use std::mem::{self, replace};
use std::ops::{Deref, DerefMut};

use types::*;

use data::Data;
use util::stable_id_map::{self, StableIdMap, Stable};
use world::types::*;
use world::ops::{self, OpResult};
use world::object::{Object, ObjectRef, ObjectRefMut};
use world::hooks::{Hooks, NoHooks};


impl<'d> super::World<'d> {
    pub fn new(data: &'d Data) -> World<'d> {
        World {
            data: data,
            journal: Vec::new(),

            clients: StableIdMap::new(),
            terrain_chunks: HashMap::new(),
            entities: StableIdMap::new(),
            structures: StableIdMap::new(),
            inventories: StableIdMap::new(),

            structures_by_chunk: HashMap::new(),
        }
    }

    pub fn data(&self) -> &'d Data {
        self.data
    }

    pub fn hook<'a, H: Hooks>(&'a mut self, h: &'a mut H) -> (&'a mut World<'d>, &'a mut H) {
        (self, h)
    }

    pub fn record(&mut self, update: Update) {
        self.journal.push(update);
    }

    pub fn take_journal(&mut self) -> Vec<Update> {
        replace(&mut self.journal, Vec::new())
    }

    pub fn process_journal<D, F>(mut self_: D, mut f: F)
            where D: Deref<Target=World<'d>>+DerefMut,
                  F: FnMut(&mut D, Update) {
        let mut journal = replace(&mut self_.journal, Vec::new());
        for update in journal.drain() {
            f(&mut self_, update);
        }
        // Try to put back the original journal, to avoid an allocation.  But if the callback added
        // journal entries, skip it - we've already allocated, and we don't want to lose the new
        // entries.
        if self_.journal.len() == 0 {
            self_.journal = journal;
        }
    }


    pub fn chunk_structures<'a>(&'a self, chunk_id: V2) -> ChunkStructures<'a, 'd> {
        ChunkStructures {
            world: self,
            iter: self.structures_by_chunk.get(&chunk_id).map(|xs| xs.iter()),
        }
    }

    pub fn clients<'a>(&'a self) -> Clients<'a, 'd> {
        Clients {
            world: self,
            iter: self.clients.iter(),
        }
    }

    pub fn terrain_chunks<'a>(&'a self) -> TerrainChunksById<'a, 'd, hash_map::Keys<'a, V2, TerrainChunk>> {
        TerrainChunksById {
            world: self,
            iter: self.terrain_chunks.keys(),
        }
    }

    pub fn entities<'a>(&'a self) -> Entities<'a, 'd> {
        Entities {
            world: self,
            iter: self.entities.iter(),
        }
    }

    pub fn structures<'a>(&'a self) -> Structures<'a, 'd> {
        Structures {
            world: self,
            iter: self.structures.iter(),
        }
    }

    pub fn inventories<'a>(&'a self) -> Inventories<'a, 'd> {
        Inventories {
            world: self,
            iter: self.inventories.iter(),
        }
    }

}

macro_rules! process_objects {
    ($m:ident ! $($args:tt)*) => {
        $m!($($args)*
            object Client {
                id ClientId;
                map clients;
                lifecycle (name: &str)
                    create_client => client_create [id -> id],
                    destroy_client => client_destroy,
                    create_client_hooks, destroy_client_hooks;
                lookups [id -> id]
                    get_client, client,
                    get_client_mut, client_mut,
                    get_client_mut_hooks, client_mut_hooks;
                stable_ids
                    transient_client_id;
            }

            object TerrainChunk {
                id V2;
                map terrain_chunks;
                lifecycle (pos: V2, blocks: Box<BlockChunk>)
                    create_terrain_chunk => terrain_chunk_create [id -> pos],
                    destroy_terrain_chunk => terrain_chunk_destroy,
                    create_terrain_chunk_hooks, destroy_terrain_chunk_hooks;
                lookups [id -> &id]
                    get_terrain_chunk, terrain_chunk,
                    get_terrain_chunk_mut, terrain_chunk_mut,
                    get_terrain_chunk_mut_hooks, terrain_chunk_mut_hooks;
            }

            object Entity {
                id EntityId;
                map entities;
                lifecycle (pos: V3, anim: AnimId, appearance: u32)
                    create_entity => entity_create [id -> id],
                    destroy_entity => entity_destroy,
                    create_entity_hooks, destroy_entity_hooks;
                lookups [id -> id]
                    get_entity, entity,
                    get_entity_mut, entity_mut,
                    get_entity_mut_hooks, entity_mut_hooks;
                stable_ids
                    transient_entity_id;
            }

            object Structure {
                id StructureId;
                map structures;
                lifecycle (pos: V3, tid: TemplateId)
                    create_structure => structure_create [id -> id],
                    destroy_structure => structure_destroy,
                    create_structure_hooks, destroy_structure_hooks;
                lookups [id -> id]
                    get_structure, structure,
                    get_structure_mut, structure_mut,
                    get_structure_mut_hooks, structure_mut_hooks;
                stable_ids
                    transient_structure_id;
            }

            object Inventory {
                id InventoryId;
                map inventories;
                lifecycle ()
                    create_inventory => inventory_create [id -> id],
                    destroy_inventory => inventory_destroy,
                    create_inventory_hooks, destroy_inventory_hooks;
                lookups [id -> id]
                    get_inventory, inventory,
                    get_inventory_mut, inventory_mut,
                    get_inventory_mut_hooks, inventory_mut_hooks;
                stable_ids
                    transient_inventory_id;
            }
        );
    };
}

macro_rules! world_methods {
    ($(
        object $Obj:ident {
            id $Id:ident;
            map $objs:ident;
            lifecycle ($($create_arg:ident: $create_arg_ty:ty),*)
                $create_obj:ident => $create_obj_op:ident
                    [$create_id_name:ident -> $create_id_expr:expr],
                $destroy_obj:ident => $destroy_obj_op:ident,
                $create_obj_hooks:ident, $destroy_obj_hooks:ident;
            lookups [$lookup_id_name:ident -> $lookup_id_expr:expr]
                $get_obj:ident, $obj:ident,
                $get_obj_mut:ident, $obj_mut:ident,
                $get_obj_mut_hooks:ident, $obj_mut_hooks:ident;
            $(stable_ids
                $transient_obj_id:ident;)*
        }
    )*) => {
        impl<'d> World<'d> { $(
                /*
            pub fn $create_obj<'a>(&'a mut self,
                                   $($create_arg: $create_arg_ty,)*)
                                   -> OpResult<ObjectRefMut<'a, 'd, $Obj>> {
                self.$create_obj_hooks(no_hooks(), $($create_arg,)*)
            }

            pub fn $create_obj_hooks<'a, H>(&'a mut self,
                                            h: &'a mut H,
                                            $($create_arg: $create_arg_ty,)*)
                                            -> OpResult<ObjectRefMut<'a, 'd, $Obj, H>>
                    where H: Hooks {
                let $create_id_name = try!(ops::$create_obj_op(self, h, $($create_arg,)*));
                Ok(ObjectRefMut {
                    world: self,
                    hooks: h,
                    id: $create_id_expr,
                })
            }

            pub fn $destroy_obj(&mut self, id: $Id) -> OpResult<()> {
                self.$destroy_obj_hooks(no_hooks(), id)
            }

            pub fn $destroy_obj_hooks<H>(&mut self, h: &mut H, id: $Id) -> OpResult<()>
                    where H: Hooks {
                ops::$destroy_obj_op(self, h, id)
            }
            */


            pub fn $get_obj<'a>(&'a self,
                                $lookup_id_name: $Id) -> Option<ObjectRef<'a, 'd, $Obj>> {
                let obj = match self.$objs.get($lookup_id_expr) {
                    None => return None,
                    Some(x) => x,
                };

                Some(ObjectRef {
                    world: self,
                    id: $lookup_id_name,
                    obj: obj,
                })
            }

            pub fn $obj<'a>(&'a self, id: $Id) -> ObjectRef<'a, 'd, $Obj> {
                self.$get_obj(id)
                    .expect(concat!("no ", stringify!($Obj), " with given id"))
            }

            /*
            pub fn $get_obj_mut<'a>(&'a mut self, id: $Id)
                                    -> Option<ObjectRefMut<'a, 'd, $Obj>> {
                self.$get_obj_mut_hooks(no_hooks(), id)
            }

            pub fn $obj_mut<'a>(&'a mut self, id: $Id)
                                -> ObjectRefMut<'a, 'd, $Obj> {
                self.$obj_mut_hooks(no_hooks(), id)
            }

            pub fn $get_obj_mut_hooks<'a, H>(&'a mut self,
                                             h: &'a mut H,
                                             $lookup_id_name: $Id)
                                             -> Option<ObjectRefMut<'a, 'd, $Obj, H>>
                    where H: Hooks {
                // Check that the ID is valid.
                match self.$objs.get($lookup_id_expr) {
                    None => return None,
                    Some(_) => {},
                }

                Some(ObjectRefMut {
                    world: self,
                    hooks: h,
                    id: $lookup_id_name,
                })
            }

            pub fn $obj_mut_hooks<'a, H>(&'a mut self,
                                         h: &'a mut H,
                                         id: $Id)
                                         -> ObjectRefMut<'a, 'd, $Obj, H>
                    where H: Hooks {
                self.$get_obj_mut_hooks(h, id)
                    .expect(concat!("no ", stringify!($Obj), " with given id"))
            }

            */

            $(
                pub fn $transient_obj_id(&self, stable_id: Stable<$Id>) -> Option<$Id> {
                    self.$objs.get_id(stable_id)
                }
            )*

        )* }
    }
}

process_objects!(world_methods!);

pub struct ChunkStructures<'a, 'd: 'a> {
    world: &'a World<'d>,
    iter: Option<hash_set::Iter<'a, StructureId>>,
}

impl<'a, 'd> Iterator for ChunkStructures<'a, 'd> {
    type Item = ObjectRef<'a, 'd, Structure>;
    fn next(&mut self) -> Option<ObjectRef<'a, 'd, Structure>> {
        let iter = match self.iter {
            Some(ref mut x) => x,
            None => return None,
        };

        let world = self.world;
        iter.next().map(|&sid| {
            let s = &world.structures[sid];
            ObjectRef {
                world: world,
                id: sid,
                obj: s,
            }
        })
    }
}


macro_rules! object_iter {
    ($name:ident, $obj_ty:ty, $id_ty:ty) => {
        pub struct $name<'a, 'd: 'a> {
            world: &'a World<'d>,
            iter: stable_id_map::Iter<'a, $id_ty, $obj_ty>,
        }

        impl<'a, 'd> Iterator for $name<'a, 'd> {
            type Item = ObjectRef<'a, 'd, $obj_ty>;
            fn next(&mut self) -> Option<ObjectRef<'a, 'd, $obj_ty>> {
                let world = self.world;
                self.iter.next().map(|(oid, o)| {
                    ObjectRef {
                        world: world,
                        id: oid,
                        obj: o,
                    }
                })
            }
        }
    };
}

object_iter!(Clients, Client, ClientId);
object_iter!(Entities, Entity, EntityId);
object_iter!(Structures, Structure, StructureId);
object_iter!(Inventories, Inventory, InventoryId);


macro_rules! object_iter_by_id {
    ($name:ident, $obj_ty:ty, $id_ty:ty) => {
        pub struct $name<'a, 'd: 'a, I> {
            world: &'a World<'d>,
            iter: I,
        }

        impl<'a, 'd, 'b, I: Iterator<Item=&'b $id_ty>> $name<'a, 'd, I> {
            pub fn new(world: &'a World<'d>, iter: I) -> $name<'a, 'd, I> {
                $name {
                    world: world,
                    iter: iter,
                }
            }
        }

        impl<'a, 'd, 'b, I: Iterator<Item=&'b $id_ty>> Iterator for $name<'a, 'd, I> {
            type Item = ObjectRef<'a, 'd, $obj_ty>;
            fn next(&mut self) -> Option<ObjectRef<'a, 'd, $obj_ty>> {
                let world = self.world;
                self.iter.next().map(|&oid| {
                    ObjectRef {
                        world: world,
                        id: oid,
                        obj: <$obj_ty as Object>::get(world, oid).unwrap(),
                    }
                })
            }
        }
    };
}

object_iter_by_id!(ClientsById, Client, ClientId);
object_iter_by_id!(TerrainChunksById, TerrainChunk, V2);
object_iter_by_id!(EntitiesById, Entity, EntityId);
object_iter_by_id!(StructuresById, Structure, StructureId);
object_iter_by_id!(InventoriesById, Inventory, InventoryId);
