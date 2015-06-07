use std::borrow::ToOwned;

use physics::CHUNK_SIZE;

use types::*;
use util::StrResult;

use engine::Engine;
use engine::glue::WorldFragment;
use logic;
use lua::LuaState;
use messages::ClientResponse;
use msg;
use script::traits::Userdata;
use script::userdata::TakeOptWrapper;
use script::userdata::extra_arg::ExtraArg;
use world;
use world::Fragment;
use world::object::*;


#[derive(Clone, Copy)]
pub struct World;

impl_type_name!(World);
impl_metatable_key!(World);
impl_fromlua_copy!(World);

impl Userdata for World {
    fn populate_table(lua: &mut LuaState) {
        lua_table_fns2! {
            lua, -1,

            fn get() -> World {
                World
            }

            fn create_entity(!partial wf: WorldFragment,
                             _w: World,
                             plane: Plane,
                             pos: V3,
                             anim: AnimId,
                             appearance: u32) -> StrResult<Entity> {
                let stable_pid = wf.plane_mut(plane.id).stable_id();
                wf.create_entity(stable_pid, pos, anim, appearance)
                  .map(|e| Entity { id: e.id() })
            }

            fn create_plane(!partial wf: WorldFragment,
                            _w: World,
                            name: &str) -> StrResult<Plane> {
                wf.create_plane(name.to_owned())
                  .map(|p| Plane { id: p.id() })
            }

            fn get_forest_plane(_w: World) -> StablePlane {
                StablePlane { id: STABLE_PLANE_FOREST }
            }

            fn find_structure_at_point(!partial w: &world::World,
                                       _w: World,
                                       plane: Plane,
                                       pos: V3) -> Option<Structure> {
                let chunk = pos.reduce().div_floor(scalar(CHUNK_SIZE));
                let mut best_id = None;
                let mut best_layer = 0;
                for s in w.chunk_structures(plane.id, chunk) {
                    if s.bounds().contains(pos) {
                        if s.template().layer >= best_layer {
                            best_layer = s.template().layer;
                            best_id = Some(s.id());
                        }
                    }
                };
                best_id.map(|sid| Structure { id: sid })
            }

            fn find_structure_at_point_layer(!partial w: &world::World,
                                             _w: World,
                                             plane: Plane,
                                             pos: V3,
                                             layer: u8) -> Option<Structure> {
                let chunk = pos.reduce().div_floor(scalar(CHUNK_SIZE));
                for s in w.chunk_structures(plane.id, chunk) {
                    if s.bounds().contains(pos) && s.template().layer == layer {
                        return Some(Structure { id: s.id() });
                    }
                };
                None
            }

            fn create_structure(!partial wf: WorldFragment,
                                _w: World,
                                plane: Plane,
                                pos: V3,
                                template_name: &str) -> StrResult<Structure> {{
                let template_id =
                    unwrap!(wf.data().structure_templates.find_id(template_name),
                            "named structure template does not exist");

                let mut s = try!(wf.create_structure(plane.id, pos, template_id));
                try!(s.set_attachment(world::StructureAttachment::Chunk));
                Ok(Structure { id: s.id() })
            }}

            fn create_inventory(!partial wf: WorldFragment, _w: World) -> StrResult<Inventory> {
                wf.create_inventory()
                  .map(|i| Inventory { id: i.id() })
            }

            fn item_id_to_name(!partial w: &world::World, _w: World, id: ItemId) -> _ {
                w.data().item_data.get_name(id).map(|s| s.to_owned())
            }

            fn item_name_to_id(!partial w: &world::World, _w: World, name: &str) -> _ {
                w.data().item_data.find_id(name)
            }

            fn get_client(!partial w: &world::World,
                          _w: World,
                          id: ClientId) -> Option<Client> {
                w.get_client(id).map(|_| Client { id: id })
            }

            fn get_entity(!partial w: &world::World,
                          _w: World,
                          id: EntityId) -> Option<Entity> {
                w.get_entity(id).map(|_| Entity { id: id })
            }

            fn get_structure(!partial w: &world::World,
                             _w: World,
                             id: StructureId) -> Option<Structure> {
                w.get_structure(id).map(|_| Structure { id: id })
            }

            fn get_inventory(!partial w: &world::World,
                             _w: World,
                             id: InventoryId) -> Option<Inventory> {
                w.get_inventory(id).map(|_| Inventory { id: id })
            }
        }
    }
}


#[derive(Clone, Copy)]
pub struct Client {
    pub id: ClientId,
}

impl_type_name!(Client);
impl_metatable_key!(Client);
impl_fromlua_copy!(Client);

impl Userdata for Client {
    fn populate_table(lua: &mut LuaState) {
        lua_table_fns2! {
            lua, -1,

            fn world(_c: Client) -> World { World }
            fn id(c: Client) -> u16 { c.id.unwrap() }

            fn stable_id(!partial wf: WorldFragment, c: Client) -> Option<StableClient> {
                wf.get_client_mut(c.id)
                  .map(|mut c| StableClient { id: c.stable_id() })
            }

            fn name(!partial w: &world::World, c: Client) -> Option<String> {
                w.get_client(c.id)
                 .map(|c| c.name().to_owned())
            }

            fn pawn(!partial w: &world::World, c: Client) -> Option<Entity> {
                w.get_client(c.id)
                 .and_then(|c| c.pawn_id())
                 .map(|eid| Entity { id: eid })
            }

            fn set_pawn(!partial wf: WorldFragment, c: Client, e: Entity) -> StrResult<()> {
                let mut c = unwrap!(wf.get_client_mut(c.id));
                try!(c.set_pawn(Some(e.id)));
                Ok(())
            }

            fn clear_pawn(!partial wf: WorldFragment, c: Client) -> StrResult<()> {
                let mut c = unwrap!(wf.get_client_mut(c.id));
                try!(c.set_pawn(None));
                Ok(())
            }

            fn open_inventory(!full eng: &mut Engine,
                              c: Client,
                              i: Inventory) -> StrResult<()> {
                logic::items::open_inventory(eng.as_ref(), c.id, i.id)
            }

            fn open_container(!full eng: &mut Engine,
                              c: Client,
                              i1: Inventory,
                              i2: Inventory) -> StrResult<()> {
                logic::items::open_container(eng.as_ref(), c.id, i1.id, i2.id)
            }

            fn open_crafting(!full eng: &mut Engine,
                             c: Client,
                             s: Structure,
                             i: Inventory) -> StrResult<()> {
                logic::items::open_crafting(eng.as_ref(), c.id, s.id, i.id)
            }

            fn set_main_inventories(!full eng: &mut Engine,
                                    c: Client,
                                    item_inv: Inventory,
                                    ability_inv: Inventory) -> StrResult<()> {
                logic::items::set_main_inventories(eng.as_ref(),
                                                   c.id,
                                                   item_inv.id,
                                                   ability_inv.id)
            }

            fn send_message_raw(!full eng: &mut Engine,
                                c: Client,
                                msg: String) -> StrResult<()> {
                unwrap!(eng.world.get_client(c.id));
                let resp = ClientResponse::ChatUpdate(msg);
                eng.messages.send_client(c.id, resp);
                Ok(())
            }

            fn get_interact_args(!full eng: &mut Engine,
                                 c: Client,
                                 dialog_id: u32,
                                 parts: TakeOptWrapper<msg::ExtraArg>) -> StrResult<()> {
                let parts = unwrap!(parts.0);
                unwrap!(eng.world.get_client(c.id));
                let resp = ClientResponse::GetInteractArgs(dialog_id, parts);
                eng.messages.send_client(c.id, resp);
                Ok(())
            }

            fn get_use_item_args(!full eng: &mut Engine,
                                 c: Client,
                                 item_id: ItemId,
                                 dialog_id: u32,
                                 parts: TakeOptWrapper<msg::ExtraArg>) -> StrResult<()> {
                let parts = unwrap!(parts.0);
                unwrap!(eng.world.get_client(c.id));
                let resp = ClientResponse::GetUseItemArgs(item_id, dialog_id, parts);
                eng.messages.send_client(c.id, resp);
                Ok(())
            }

            fn get_use_ability_args(!full eng: &mut Engine,
                                    c: Client,
                                    item_id: ItemId,
                                    dialog_id: u32,
                                    parts: TakeOptWrapper<msg::ExtraArg>) -> StrResult<()> {
                let parts = unwrap!(parts.0);
                unwrap!(eng.world.get_client(c.id));
                let resp = ClientResponse::GetUseAbilityArgs(item_id, dialog_id, parts);
                eng.messages.send_client(c.id, resp);
                Ok(())
            }
        }
    }
}



#[derive(Clone, Copy)]
pub struct Entity {
    pub id: EntityId,
}

impl_type_name!(Entity);
impl_metatable_key!(Entity);
impl_fromlua_copy!(Entity);

impl Userdata for Entity {
    fn populate_table(lua: &mut LuaState) {
        use world::EntityAttachment;

        lua_table_fns2! {
            lua, -1,

            fn world(_e: Entity) -> World { World }
            fn id(e: Entity) -> u32 { e.id.unwrap() }

            fn stable_id(!partial wf: WorldFragment,
                         e: Entity) -> Option<StableEntity> {
                wf.get_entity_mut(e.id)
                  .map(|mut e| StableEntity { id: e.stable_id() })
            }

            fn destroy(!partial wf: WorldFragment,
                       e: Entity) -> StrResult<()> {
                wf.destroy_entity(e.id)
            }

            fn plane(!partial w: &world::World, e: Entity) -> Option<Plane> {
                w.get_entity(e.id)
                 .map(|e| Plane { id: e.plane_id() })
            }

            fn pos(!partial wf: WorldFragment, e: Entity) -> Option<V3> {
                let now = wf.now();
                wf.world().get_entity(e.id).map(|e| e.pos(now))
            }

            fn facing(!partial w: &world::World, e: Entity) -> Option<V3> {
                w.get_entity(e.id).map(|e| e.facing())
            }

            fn get_appearance(!partial w: &world::World,
                              e: Entity) -> Option<u32> {
                w.get_entity(e.id)
                 .map(|e| e.appearance())
            }

            fn get_appearance_bits(!partial w: &world::World,
                                   e: Entity,
                                   mask: u32) -> Option<u32> {
                w.get_entity(e.id)
                 .map(|e| e.appearance() & mask)
            }

            fn update_appearance(!partial wf: WorldFragment,
                                 e: Entity,
                                 mask: u32,
                                 bits: u32) -> StrResult<()> {
                let mut e = unwrap!(wf.get_entity_mut(e.id));
                let appearance = e.appearance() & !mask | bits;
                e.set_appearance(appearance);
                Ok(())
            }


            fn teleport(!partial wf: WorldFragment,
                        e: Entity,
                        pos: V3) -> StrResult<()> {
                let now = wf.now();
                let mut e = unwrap!(wf.get_entity_mut(e.id));
                e.set_motion(world::Motion::stationary(pos, now));
                Ok(())
            }

            fn teleport_plane(!partial wf: WorldFragment,
                              e: Entity,
                              p: Plane,
                              pos: V3) -> StrResult<()> {
                let now = wf.now();
                let mut e = unwrap!(wf.get_entity_mut(e.id));
                try!(e.set_plane_id(p.id));
                e.set_motion(world::Motion::stationary(pos, now));
                Ok(())
            }

            fn teleport_stable_plane(!partial wf: WorldFragment,
                                     e: Entity,
                                     p: StablePlane,
                                     pos: V3) -> StrResult<()> {
                let now = wf.now();
                let mut e = unwrap!(wf.get_entity_mut(e.id));
                try!(e.set_stable_plane_id(p.id));
                e.set_motion(world::Motion::stationary(pos, now));
                Ok(())
            }

            // TODO: come up with a lua representation of attachment so we can unify these methods
            // and also return the previous attachment (like the underlying op does)
            fn attach_to_world(!partial wf: WorldFragment,
                               e: Entity) -> StrResult<()> {
                let mut e = unwrap!(wf.get_entity_mut(e.id));
                try!(e.set_attachment(EntityAttachment::World));
                Ok(())
            }

            fn attach_to_client(!partial wf: WorldFragment,
                                e: Entity,
                                c: Client) -> StrResult<()> {
                let mut e = unwrap!(wf.get_entity_mut(e.id));
                try!(e.set_attachment(EntityAttachment::Client(c.id)));
                Ok(())
            }
        }
    }
}


#[derive(Clone, Copy)]
pub struct Inventory {
    pub id: InventoryId,
}

impl_type_name!(Inventory);
impl_metatable_key!(Inventory);
impl_fromlua_copy!(Inventory);

impl Userdata for Inventory {
    fn populate_table(lua: &mut LuaState) {
        use world::InventoryAttachment;

        lua_table_fns2! {
            lua, -1,

            fn world(_i: Inventory) -> World { World }
            fn id(i: Inventory) -> u32 { i.id.unwrap() }

            fn stable_id(!partial wf: WorldFragment, i: Inventory) -> Option<StableInventory> {
                wf.get_inventory_mut(i.id)
                  .map(|mut i| StableInventory { id: i.stable_id() })
            }

            fn destroy(!partial wf: WorldFragment, i: Inventory) -> StrResult<()> {
                wf.destroy_inventory(i.id)
            }

            fn count(!partial w: &world::World, i: Inventory, name: &str) -> StrResult<u8> {
                let i = unwrap!(w.get_inventory(i.id));
                i.count_by_name(name)
            }

            fn update(!partial wf: WorldFragment,
                      i: Inventory,
                      name: &str,
                      adjust: i16) -> StrResult<u8> {
                let mut i = unwrap!(wf.get_inventory_mut(i.id));
                i.update_by_name(name, adjust)
            }

            fn attach_to_world(!partial wf: WorldFragment,
                               i: Inventory) -> StrResult<()> {
                let mut i = unwrap!(wf.get_inventory_mut(i.id));
                try!(i.set_attachment(InventoryAttachment::World));
                Ok(())
            }

            fn attach_to_client(!partial wf: WorldFragment,
                                i: Inventory,
                                c: Client) -> StrResult<()> {
                let mut i = unwrap!(wf.get_inventory_mut(i.id));
                try!(i.set_attachment(InventoryAttachment::Client(c.id)));
                Ok(())
            }

            fn attach_to_entity(!partial wf: WorldFragment,
                                i: Inventory,
                                e: Entity) -> StrResult<()> {
                let mut i = unwrap!(wf.get_inventory_mut(i.id));
                try!(i.set_attachment(InventoryAttachment::Entity(e.id)));
                Ok(())
            }

            fn attach_to_structure(!partial wf: WorldFragment,
                                   i: Inventory,
                                   s: Structure) -> StrResult<()> {
                let mut i = unwrap!(wf.get_inventory_mut(i.id));
                try!(i.set_attachment(InventoryAttachment::Structure(s.id)));
                Ok(())
            }
        }
    }
}


#[derive(Clone, Copy)]
pub struct Plane {
    pub id: PlaneId,
}

impl_type_name!(Plane);
impl_metatable_key!(Plane);
impl_fromlua_copy!(Plane);

impl Userdata for Plane {
    fn populate_table(lua: &mut LuaState) {
        lua_table_fns2! {
            lua, -1,

            fn world(_p: Plane) -> World { World }
            fn id(p: Plane) -> u32 { p.id.unwrap() }

            fn stable_id(!partial wf: WorldFragment, p: Plane) -> Option<StablePlane> {
                wf.get_plane_mut(p.id)
                  .map(|mut p| StablePlane { id: p.stable_id() })
            }

            fn name(!partial w: &world::World, p: Plane) -> Option<String> {
                w.get_plane(p.id)
                 .map(|p| p.name().to_owned())
            }

            fn set_interior(!partial wf: WorldFragment,
                            plane: Plane,
                            pos: V3,
                            base: &str) -> StrResult<()> {
                logic::misc::set_block_interior(&mut wf, plane.id, pos, base)
            }

            fn clear_interior(!partial wf: WorldFragment,
                              plane: Plane,
                              pos: V3,
                              base: &str,
                              new_center: &str) -> StrResult<()> {
                let new_center_id = unwrap!(wf.data().block_data.find_id(new_center));
                logic::misc::clear_block_interior(&mut wf, plane.id, pos, base, new_center_id)
            }

            fn get_block(!partial w: &world::World,
                         plane: Plane,
                         pos: V3) -> Option<String> {
                let p = unwrap_or!(w.get_plane(plane.id), return None);
                let cpos = pos.reduce().div_floor(scalar(CHUNK_SIZE));
                let tc = unwrap_or!(p.get_terrain_chunk(cpos), return None);
                let idx = tc.bounds().index(pos);
                let block_id = tc.blocks()[idx];
                Some(w.data().block_data.name(block_id).to_owned())
            }
        }
    }
}


#[derive(Clone, Copy)]
pub struct Structure {
    pub id: StructureId,
}

impl_type_name!(Structure);
impl_metatable_key!(Structure);
impl_fromlua_copy!(Structure);

impl Userdata for Structure {
    fn populate_table(lua: &mut LuaState) {
        use world::StructureAttachment;

        lua_table_fns2! {
            lua, -1,

            fn world(_s: Structure) -> World { World }
            fn id(s: Structure) -> u32 { s.id.unwrap() }

            fn stable_id(!partial wf: WorldFragment, s: Structure) -> Option<StableStructure> {
                wf.get_structure_mut(s.id)
                  .map(|mut s| StableStructure { id: s.stable_id() })
            }

            fn destroy(!partial wf: WorldFragment, s: Structure) -> StrResult<()> {
                wf.destroy_structure(s.id)
            }

            fn plane(!partial w: &world::World, s: Structure) -> Option<Plane> {
                w.get_structure(s.id)
                 .map(|s| Plane { id: s.plane_id() })
            }

            fn pos(!partial w: &world::World, s: Structure) -> Option<V3> {
                w.get_structure(s.id)
                 .map(|s| s.pos())
            }

            fn size(!partial w: &world::World, s: Structure) -> Option<V3> {
                w.get_structure(s.id)
                 .map(|s| s.size())
            }

            fn template_id(!partial w: &world::World, s: Structure) -> Option<u32> {
                w.get_structure(s.id)
                 .map(|s| s.template_id())
            }

            fn template(!partial w: &world::World, s: Structure) -> Option<String> {
                w.get_structure(s.id)
                 .map(|s| s.template_id())
                 .and_then(|id| w.data().structure_templates.get_template(id))
                 .map(|t| t.name.clone())
            }

            fn layer(!partial w: &world::World, s: Structure) -> Option<u8> {
                w.get_structure(s.id)
                 .map(|s| s.template_id())
                 .and_then(|id| w.data().structure_templates.get_template(id))
                 .map(|t| t.layer)
            }

            fn replace(!partial wf: WorldFragment,
                       s: Structure,
                       new_template_name: &str) -> StrResult<()> {
                let new_template_id =
                    unwrap!(wf.data().structure_templates.find_id(new_template_name),
                            "named structure template does not exist");

                let mut s = unwrap!(wf.get_structure_mut(s.id));
                s.set_template_id(new_template_id)
            }

            fn set_has_save_hooks(!partial wf: WorldFragment,
                                  s: Structure,
                                  set: bool) -> StrResult<()> {
                let mut s = unwrap!(wf.get_structure_mut(s.id));
                let flags = s.flags();
                if set {
                    s.set_flags(flags | world::flags::S_HAS_SAVE_HOOKS);
                } else {
                    s.set_flags(flags - world::flags::S_HAS_SAVE_HOOKS);
                };
                Ok(())
            }

            fn attach_to_plane(!partial wf: WorldFragment, s: Structure) -> StrResult<()> {
                let mut s = unwrap!(wf.get_structure_mut(s.id));
                try!(s.set_attachment(StructureAttachment::Plane));
                Ok(())
            }

            fn attach_to_chunk(!partial wf: WorldFragment, s: Structure) -> StrResult<()> {
                let mut s = unwrap!(wf.get_structure_mut(s.id));
                try!(s.set_attachment(StructureAttachment::Chunk));
                Ok(())
            }
        }
    }
}


macro_rules! define_stable_wrapper {
    ($name:ident, $obj_ty:ident, $id_ty:ty, $transient_id:ident) => {
        #[derive(Clone, Copy)]
        pub struct $name {
            pub id: Stable<$id_ty>,
        }

        impl_type_name!($name);
        impl_metatable_key!($name);
        impl_fromlua_copy!($name);

        impl Userdata for $name {
            fn populate_table(lua: &mut LuaState) {
                lua_table_fns2! {
                    lua, -1,

                    fn id(stable: $name) -> String {
                        format!("{:x}", stable.id.val)
                    }

                    fn get(!partial w: &world::World, stable: $name) -> Option<$obj_ty> {
                        w.$transient_id(stable.id)
                         .map(|id| $obj_ty { id: id })
                    }
                }
            }

            fn populate_metatable(lua: &mut LuaState) {
                lua_table_fns2! {
                    lua, -1,

                    fn __eq(a: $name, b: $name) -> bool {
                        a.id == b.id
                    }
                }
            }
        }
    };
}

define_stable_wrapper!(StableClient, Client, ClientId, transient_client_id);
define_stable_wrapper!(StableEntity, Entity, EntityId, transient_entity_id);
define_stable_wrapper!(StableInventory, Inventory, InventoryId, transient_inventory_id);
define_stable_wrapper!(StablePlane, Plane, PlaneId, transient_plane_id);
define_stable_wrapper!(StableStructure, Structure, StructureId, transient_structure_id);
