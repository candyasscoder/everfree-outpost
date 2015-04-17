use std::collections::{HashMap, hash_map, hash_set};

use types::*;

use data::Data;
use util::stable_id_map::{self, StableIdMap};
use world::types::*;
use world::object::{Object, ObjectRef};


impl<'d> super::World<'d> {
    pub fn new(data: &'d Data) -> World<'d> {
        World {
            data: data,

            clients: StableIdMap::new(),
            entities: StableIdMap::new(),
            inventories: StableIdMap::new(),
            planes: StableIdMap::new(),
            terrain_chunks: StableIdMap::new(),
            structures: StableIdMap::new(),

            structures_by_chunk: HashMap::new(),
        }
    }

    pub fn data(&self) -> &'d Data {
        self.data
    }


    pub fn get_chunk<'a>(&'a self, pid: PlaneId, cpos: V2)
                         -> Option<ObjectRef<'a, 'd, TerrainChunk>> {
        let p = unwrap_or!(self.planes.get(pid), return None);
        let &tcid = unwrap_or!(p.loaded_chunks.get(&cpos), return None);
        // `tcid` came from a plane's `loaded_chunks`, so it must refer to an existing
        // TerrainChunk.
        let tc = self.terrain_chunks.get(tcid).unwrap();

        Some(ObjectRef {
            world: self,
            id: tcid,
            obj: tc,
        })
    }


    pub fn chunk_structures<'a>(&'a self, pid: PlaneId, cpos: V2) -> ChunkStructures<'a, 'd> {
        ChunkStructures {
            world: self,
            iter: self.structures_by_chunk.get(&(pid, cpos)).map(|xs| xs.iter()),
        }
    }

    pub fn clients<'a>(&'a self) -> Clients<'a, 'd> {
        Clients {
            world: self,
            iter: self.clients.iter(),
        }
    }

    pub fn entities<'a>(&'a self) -> Entities<'a, 'd> {
        Entities {
            world: self,
            iter: self.entities.iter(),
        }
    }

    pub fn inventories<'a>(&'a self) -> Inventories<'a, 'd> {
        Inventories {
            world: self,
            iter: self.inventories.iter(),
        }
    }

    pub fn planes<'a>(&'a self) -> Planes<'a, 'd> {
        Planes {
            world: self,
            iter: self.planes.iter(),
        }
    }

    pub fn terrain_chunks<'a>(&'a self) -> TerrainChunks<'a, 'd> {
        TerrainChunks {
            world: self,
            iter: self.terrain_chunks.iter(),
        }
    }

    pub fn structures<'a>(&'a self) -> Structures<'a, 'd> {
        Structures {
            world: self,
            iter: self.structures.iter(),
        }
    }

}

macro_rules! process_objects {
    ($m:ident ! $($args:tt)*) => {
        $m!($($args)*
            object Client {
                id ClientId;
                map clients;
                module client;
                lifecycle (name: &str)
                    create_client [id -> id],
                    destroy_client,
                lookups [id -> id]
                    get_client, client,
                    get_client_mut, client_mut,
                stable_ids
                    transient_client_id;
            }

            object Entity {
                id EntityId;
                map entities;
                module entity;
                lifecycle (plane: Stable<PlaneId>, pos: V3, anim: AnimId, appearance: u32)
                    create_entity [id -> id],
                    destroy_entity,
                lookups [id -> id]
                    get_entity, entity,
                    get_entity_mut, entity_mut,
                stable_ids
                    transient_entity_id;
            }

            object Inventory {
                id InventoryId;
                map inventories;
                module inventory;
                lifecycle ()
                    create_inventory [id -> id],
                    destroy_inventory,
                lookups [id -> id]
                    get_inventory, inventory,
                    get_inventory_mut, inventory_mut,
                stable_ids
                    transient_inventory_id;
            }

            object Plane {
                id PlaneId;
                map planes;
                module plane;
                lifecycle ()
                    create_plane [id -> id],
                    destroy_plane,
                lookups [id -> id]
                    get_plane, plane,
                    get_plane_mut, plane_mut,
            }

            object TerrainChunk {
                id TerrainChunkId;
                map terrain_chunks;
                module terrain_chunk;
                lifecycle (pid: PlaneId, cpos: V2, blocks: Box<BlockChunk>)
                    create_terrain_chunk [id -> id],
                    destroy_terrain_chunk,
                lookups [id -> id]
                    get_terrain_chunk, terrain_chunk,
                    get_terrain_chunk_mut, terrain_chunk_mut,
            }

            object Structure {
                id StructureId;
                map structures;
                module structure;
                lifecycle (pid: PlaneId, pos: V3, tid: TemplateId)
                    create_structure [id -> id],
                    destroy_structure,
                lookups [id -> id]
                    get_structure, structure,
                    get_structure_mut, structure_mut,
                stable_ids
                    transient_structure_id;
            }
        );
    };
}

macro_rules! world_methods {
    ($(
        object $Obj:ident {
            id $Id:ident;
            map $objs:ident;
            module $module:ident;
            lifecycle ($($create_arg:ident: $create_arg_ty:ty),*)
                $create_obj:ident [$create_id_name:ident -> $create_id_expr:expr],
                $destroy_obj:ident,
            lookups [$lookup_id_name:ident -> $lookup_id_expr:expr]
                $get_obj:ident, $obj:ident,
                $get_obj_mut:ident, $obj_mut:ident,
            $(stable_ids
                $transient_obj_id:ident;)*
        }
    )*) => {
        impl<'d> World<'d> { $(
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
object_iter!(Inventories, Inventory, InventoryId);
object_iter!(Planes, Plane, PlaneId);
object_iter!(TerrainChunks, TerrainChunk, TerrainChunkId);
object_iter!(Structures, Structure, StructureId);


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
object_iter_by_id!(EntitiesById, Entity, EntityId);
object_iter_by_id!(InventoriesById, Inventory, InventoryId);
object_iter_by_id!(PlaneById, Plane, PlaneId);
object_iter_by_id!(TerrainChunksById, TerrainChunk, TerrainChunkId);
object_iter_by_id!(StructuresById, Structure, StructureId);
