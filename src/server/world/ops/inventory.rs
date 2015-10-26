use std::cmp;
use std::collections::HashMap;
use std::mem::replace;
use std::u8;

use types::*;
use util;
use util::SmallVec;

use world::{Inventory, InventoryAttachment, Item};
use world::{Fragment, Hooks};
use world::ops::OpResult;


// Inventory size (number of slots) is capped at 255
pub fn create<'d, F>(f: &mut F, size: u8) -> OpResult<InventoryId>
        where F: Fragment<'d> {
    let iid = create_unchecked(f, size);
    f.with_hooks(|h| h.on_inventory_create(iid));
    Ok(iid)
}

pub fn create_unchecked<'d, F>(f: &mut F, size: u8) -> InventoryId
        where F: Fragment<'d> {
    let iid = f.world_mut().inventories.insert(Inventory {
        contents: util::make_array(Item::Empty, size as usize),

        stable_id: NO_STABLE_ID,
        attachment: InventoryAttachment::World,
    }).unwrap();     // Shouldn't fail when stable_id == NO_STABLE_ID
    iid
}

pub fn destroy<'d, F>(f: &mut F,
                      iid: InventoryId) -> OpResult<()>
        where F: Fragment<'d> {
    use world::InventoryAttachment::*;
    let i = unwrap!(f.world_mut().inventories.remove(iid));

    match i.attachment {
        World => {},
        Client(cid) => {
            if let Some(c) = f.world_mut().clients.get_mut(cid) {
                c.child_inventories.remove(&iid);
            }
        },
        Entity(eid) => {
            if let Some(e) = f.world_mut().entities.get_mut(eid) {
                e.child_inventories.remove(&iid);
            }
        },
        Structure(sid) => {
            if let Some(s) = f.world_mut().structures.get_mut(sid) {
                s.child_inventories.remove(&iid);
            }
        },
    }

    f.with_hooks(|h| h.on_inventory_destroy(iid));
    Ok(())
}

pub fn attach<'d, F>(f: &mut F,
                     iid: InventoryId,
                     new_attach: InventoryAttachment) -> OpResult<InventoryAttachment>
        where F: Fragment<'d> {
    use world::InventoryAttachment::*;

    let w = f.world_mut();
    let i = unwrap!(w.inventories.get_mut(iid));

    if new_attach == i.attachment {
        return Ok(new_attach);
    }

    match new_attach {
        World => {},
        Client(cid) => {
            let c = unwrap!(w.clients.get_mut(cid),
                            "can't attach inventory to nonexistent client");
            c.child_inventories.insert(iid);
        },
        Entity(eid) => {
            let e = unwrap!(w.entities.get_mut(eid),
                            "can't attach inventory to nonexistent entity");
            e.child_inventories.insert(iid);
        },
        Structure(sid) => {
            let s = unwrap!(w.structures.get_mut(sid),
                            "can't attach inventory to nonexistent structure");
            s.child_inventories.insert(iid);
        },
    }

    let old_attach = replace(&mut i.attachment, new_attach);

    match old_attach {
        World => {},
        Client(cid) => {
            w.clients[cid].child_inventories.remove(&iid);
        },
        Entity(eid) => {
            w.entities[eid].child_inventories.remove(&iid);
        },
        Structure(sid) => {
            w.structures[sid].child_inventories.remove(&iid);
        },
    }

    Ok(old_attach)
}

/// Try to add a number of bulk items.  Returns the actual number of items added.  Fails only if
/// `iid` is not valid.
///
/// Bulk-related function (add, remove, count) all use u16 because the max number of bulk items is
/// 255 (slots) * 255 (stack size).
pub fn bulk_add<'d, F>(f: &mut F,
                       iid: InventoryId,
                       item_id: ItemId,
                       adjust: u16) -> OpResult<u16>
        where F: Fragment<'d> {
    let mut updated_slots = SmallVec::new();
    let transferred = {
        let i = unwrap!(f.world_mut().inventories.get_mut(iid));

        // Amount transferred so far
        let mut acc = 0;
        for (idx, slot) in i.contents.iter_mut().enumerate() {
            match *slot {
                Item::Bulk(count, slot_item_id) if slot_item_id == item_id => {
                    if count < u8::MAX {
                        let delta = cmp::min((u8::MAX - count) as u16, adjust - acc) as u8;
                        // Sum never exceeds u8::MAX.
                        *slot = Item::Bulk(count + delta, item_id);
                        updated_slots.push(idx as u8);
                        acc += delta as u16;
                    }
                },
                _ => continue,
            }
        }
        acc
    };

    for &slot_idx in updated_slots.iter() {
        f.with_hooks(|h| h.on_inventory_update(iid, slot_idx));
    }

    Ok(transferred)
}

/// Try to remove a number of bulk items.  Returns the actual number of items removed.  Fails only
/// if `iid` is not valid.
pub fn bulk_remove<'d, F>(f: &mut F,
                       iid: InventoryId,
                       item_id: ItemId,
                       adjust: u16) -> OpResult<u16>
        where F: Fragment<'d> {
    let mut updated_slots = SmallVec::new();
    let transferred = {
        let i = unwrap!(f.world_mut().inventories.get_mut(iid));

        // Amount transferred so far
        let mut acc = 0;
        for (idx, slot) in i.contents.iter_mut().enumerate() {
            match *slot {
                Item::Bulk(count, slot_item_id) if slot_item_id == item_id => {
                    let delta = cmp::min(count as u16, adjust - acc) as u8;
                    if delta == count {
                        *slot = Item::Empty;
                    } else {
                        *slot = Item::Bulk(count - delta, item_id);
                    }
                    updated_slots.push(idx as u8);
                    acc += delta as u16;
                },
                _ => continue,
            }
        }
        acc
    };

    for &slot_idx in updated_slots.iter() {
        f.with_hooks(|h| h.on_inventory_update(iid, slot_idx));
    }

    Ok(transferred)
}
