use std::borrow::ToOwned;
use std::error::Error;

use physics::{CHUNK_SIZE, TILE_SIZE};

use types::*;
use util::{SmallSet, SmallVec};
use util::StrResult;

use chunks;
use engine::Engine;
use engine::glue::*;
use engine::split::{EngineRef, Part, Open};
use input::{Action, InputBits};
use messages::{ClientResponse, Dialog};
use physics_;
use script;
use terrain_gen;
use world::{self, World};
use world::object::*;
use world::save::{self, ObjectReader, ObjectWriter, ReadHooks, WriteHooks};
use vision::{self, vision_region};


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
