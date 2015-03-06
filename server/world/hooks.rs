use types::*;

use super::World;

pub trait Hooks {
    fn on_client_create(&mut self, w: &World, cid: ClientId) {}
    fn on_client_destroy(&mut self, w: &World, cid: ClientId) {}
    fn on_client_change_pawn(&mut self,
                             w: &World,
                             cid: ClientId,
                             old_pawn: Option<EntityId>,
                             new_pan: Option<EntityId>) {}

    fn on_terrain_chunk_create(&mut self, w: &World, pos: V2) {}
    fn on_terrain_chunk_destroy(&mut self, w: &World, pos: V2) {}

    fn on_entity_create(&mut self, w: &World, eid: EntityId) {}
    fn on_entity_destroy(&mut self, w: &World, eid: EntityId) {}
    fn on_entity_motion_change(&mut self, w: &World, eid: EntityId) {}

    fn on_structure_create(&mut self, w: &World, sid: StructureId) {}
    fn on_structure_destroy(&mut self, w: &World, sid: StructureId) {}

    fn on_inventory_create(&mut self, w: &World, iid: InventoryId) {}
    fn on_inventory_destroy(&mut self, w: &World, iid: InventoryId) {}
    fn on_inventory_update(&mut self,
                           w: &World,
                           iid: InventoryId,
                           item_id: ItemId,
                           old_count: u8,
                           new_count: u8) {}

    fn on_chunk_invalidate(&mut self, w: &World, pos: V2) {}
}

pub struct NoHooks;
impl Hooks for NoHooks {}

pub fn no_hooks() -> &'static mut NoHooks {
    static mut NO_HOOKS: NoHooks = NoHooks;
    unsafe { &mut NO_HOOKS }
}
