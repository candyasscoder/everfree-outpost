use std::cmp;
use std::u8;

use types::*;
use util::StrResult;

use engine::split::EngineRef;
use messages::{ClientResponse, Dialog};
use world;
use world::object::*;
use vision;


pub fn open_inventory(mut eng: EngineRef, cid: ClientId, iid: InventoryId) -> StrResult<()> {
    // Check that IDs are valid.
    unwrap!(eng.world().get_client(cid));
    unwrap!(eng.world().get_inventory(iid));

    eng.messages_mut().send_client(cid, ClientResponse::OpenDialog(Dialog::Inventory(iid)));
    vision::Fragment::subscribe_inventory(&mut eng.as_vision_fragment(), cid, iid);

    Ok(())
}

pub fn open_container(mut eng: EngineRef,
                      cid: ClientId,
                      iid1: InventoryId,
                      iid2: InventoryId) -> StrResult<()> {
    // Check that IDs are valid.
    unwrap!(eng.world().get_client(cid));
    unwrap!(eng.world().get_inventory(iid1));
    unwrap!(eng.world().get_inventory(iid1));

    eng.messages_mut().send_client(cid, ClientResponse::OpenDialog(Dialog::Container(iid1, iid2)));
    vision::Fragment::subscribe_inventory(&mut eng.as_vision_fragment(), cid, iid1);
    vision::Fragment::subscribe_inventory(&mut eng.as_vision_fragment(), cid, iid2);

    Ok(())
}

pub fn open_crafting(mut eng: EngineRef,
                     cid: ClientId,
                     sid: StructureId,
                     iid: InventoryId) -> StrResult<()> {
    // Check that IDs are valid.
    unwrap!(eng.world().get_client(cid));
    unwrap!(eng.world().get_inventory(iid));

    let template_id = {
        let s = unwrap!(eng.world().get_structure(sid));
        s.template_id()
    };

    let dialog = Dialog::Crafting(template_id, sid, iid);
    eng.messages_mut().send_client(cid, ClientResponse::OpenDialog(dialog));
    vision::Fragment::subscribe_inventory(&mut eng.as_vision_fragment(), cid, iid);

    Ok(())
}

pub fn set_main_inventory(mut eng: EngineRef, cid: ClientId, iid: InventoryId) -> StrResult<()> {
    // Check that IDs are valid.
    unwrap!(eng.world().get_client(cid));
    unwrap!(eng.world().get_inventory(iid));

    eng.messages_mut().send_client(cid, ClientResponse::MainInventory(iid));
    vision::Fragment::subscribe_inventory(&mut eng.as_vision_fragment(), cid, iid);

    Ok(())
}


pub fn move_items(mut eng: EngineRef,
                  from_iid: InventoryId,
                  to_iid: InventoryId,
                  item_id: ItemId,
                  count: u16) -> StrResult<()> {
    let real_count = {
        let world = eng.world();
        let i1 = unwrap!(world.get_inventory(from_iid));
        let i2 = unwrap!(world.get_inventory(to_iid));
        let count1 = i1.count(item_id);
        let count2 = i2.count(item_id);
        cmp::min(cmp::min(count1 as u16, (u8::MAX - count2) as u16), count) as i16
    };
    if real_count > 0 {
        // OK: inventory IDs have already been checked.
        world::Fragment::inventory_mut(&mut eng.as_world_fragment(), from_iid)
             .update(item_id, -real_count);
        world::Fragment::inventory_mut(&mut eng.as_world_fragment(), to_iid)
             .update(item_id, real_count);
    }
    Ok(())
}

pub fn craft_recipe(mut eng: EngineRef,
                    station_sid: StructureId,
                    iid: InventoryId,
                    recipe_id: RecipeId,
                    count: u16) -> StrResult<()> {
    let recipe = unwrap!(eng.world().data().recipes.get_recipe(recipe_id));

    let _ = station_sid; // TODO
    let mut wf = eng.as_world_fragment();
    let mut i = unwrap!(world::Fragment::get_inventory_mut(&mut wf, iid));

    let real_count = {
        let mut count = count as u8;

        for (&item_id, &num_required) in recipe.inputs.iter() {
            count = cmp::min(count, i.count(item_id) / num_required);
        }

        for (&item_id, &num_produced) in recipe.outputs.iter() {
            count = cmp::min(count, (u8::MAX - i.count(item_id)) / num_produced);
        }

        count as i16
    };

    if real_count > 0 {
        for (&item_id, &num_required) in recipe.inputs.iter() {
            i.update(item_id, -real_count * num_required as i16);
        }

        for (&item_id, &num_produced) in recipe.outputs.iter() {
            i.update(item_id, real_count * num_produced as i16);
        }
    }
    Ok(())
}
