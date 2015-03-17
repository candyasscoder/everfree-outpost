use types::*;

#[allow(unused_variables)]
pub trait Hooks {
    fn on_client_create(&mut self, cid: ClientId) {}
    fn on_client_destroy(&mut self, cid: ClientId) {}
    fn on_client_change_pawn(&mut self,
                             cid: ClientId,
                             old_pawn: Option<EntityId>,
                             new_pan: Option<EntityId>) {}

    fn on_terrain_chunk_create(&mut self, pos: V2) {}
    fn on_terrain_chunk_destroy(&mut self, pos: V2) {}

    fn on_entity_create(&mut self, eid: EntityId) {}
    fn on_entity_destroy(&mut self, eid: EntityId) {}
    fn on_entity_motion_change(&mut self, eid: EntityId) {}

    fn on_structure_create(&mut self, sid: StructureId) {}
    fn on_structure_destroy(&mut self, sid: StructureId) {}

    fn on_inventory_create(&mut self, iid: InventoryId) {}
    fn on_inventory_destroy(&mut self, iid: InventoryId) {}
    fn on_inventory_update(&mut self,
                           iid: InventoryId,
                           item_id: ItemId,
                           old_count: u8,
                           new_count: u8) {}

    fn on_chunk_invalidate(&mut self, pos: V2) {}
}
