use std::cmp;
use std::collections::HashMap;
use std::mem::replace;
use std::u8;

use types::*;
use util;
use util::SmallVec;

use world::{Inventory, InventoryAttachment, Item};
use world::{Fragment, Hooks, World};
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

// This read-only method is here because it goes together with `transfer_receive` and
// `transfer_commit`.
pub fn transfer_propose(w: &World,
                        iid: InventoryId,
                        slot_id: SlotId,
                        count: u8) -> OpResult<Item> {
    let i = unwrap!(w.inventories.get(iid));

    if slot_id == NO_SLOT {
        fail!("can't transfer items out of NO_SLOT");
    }
    let slot = unwrap!(i.contents.get(slot_id as usize));

    match *slot {
        Item::Empty => Ok(*slot),
        Item::Bulk(slot_count, item_id) =>
            Ok(Item::Bulk(cmp::min(count, slot_count), item_id)),
        Item::Special(_, _) => Ok(*slot),
    }
}

pub fn transfer_receive<'d, F>(f: &mut F,
                               iid: InventoryId,
                               slot_id: SlotId,
                               xfer: Item) -> OpResult<Item>
        where F: Fragment<'d> {
    // Might need to adjust slot_id before calling hooks, if it was initially NO_SLOT.
    let mut slot_id = slot_id;
    let actual =
        match xfer {
            Item::Empty => xfer,

            Item::Bulk(count, item_id) if slot_id == NO_SLOT => {
                info!("  receive: bulk_add {:?}", xfer);
                let actual = try!(bulk_add(f, iid, item_id, count as u16)) as u8;
                // bulk_add handles calling the hooks, so we can bail out immediately.
                return Ok(Item::Bulk(actual, item_id));
            },

            Item::Bulk(count, item_id) => {
                let i = unwrap!(f.world_mut().inventories.get_mut(iid));
                let slot = *unwrap!(i.contents.get(slot_id as usize));
                match slot {
                    Item::Empty => {
                        info!("  receive: fill empty with {:?}", xfer);
                        i.contents[slot_id as usize] = xfer;
                        xfer
                    },
                    Item::Bulk(slot_count, slot_item_id) => {
                        if slot_item_id != item_id {
                            // Can't stack differing items.
                            return Ok(Item::Empty);
                        }

                        let avail = u8::MAX - slot_count;
                        let actual = cmp::min(count, avail);
                        i.contents[slot_id as usize] = Item::Bulk(slot_count + actual, item_id);
                        Item::Bulk(actual, item_id)
                    },
                    Item::Special(_, _) => {
                        // Bulk and Special items don't mix.
                        Item::Empty
                    },
                }
            },

            Item::Special(extra, item_id) => {
                let i = unwrap!(f.world_mut().inventories.get_mut(iid));

                if slot_id == NO_SLOT {
                    let mut found_empty = None;
                    for (idx, slot) in i.contents.iter().enumerate() {
                        match *slot {
                            Item::Empty => {
                                found_empty = Some(idx as u8);
                                break;
                            },
                            _ => {},
                        }
                    }
                    slot_id = unwrap!(found_empty);
                }

                let slot = unwrap!(i.contents.get_mut(slot_id as usize));
                match *slot {
                    Item::Empty => {
                        *slot = xfer;
                        xfer
                    },
                    _ => {
                        Item::Empty
                    },
                }
            },
        };

    f.with_hooks(|h| h.on_inventory_update(iid, slot_id));
    Ok(actual)
}

pub fn transfer_commit<'d, F>(f: &mut F,
                              iid: InventoryId,
                              slot_id: SlotId,
                              xfer: Item) -> OpResult<()>
        where F: Fragment<'d> {
    {
        let i = unwrap!(f.world_mut().inventories.get_mut(iid));
        let slot = unwrap!(i.contents.get_mut(slot_id as usize));
        info!("  commit: remove {:?} from {:?}", xfer, *slot);

        match xfer {
            Item::Empty => {},

            Item::Bulk(count, item_id) => {
                match *slot {
                    Item::Bulk(slot_count, slot_item_id) => {
                        if item_id != slot_item_id {
                            fail!("bad transfer_commit: item IDs don't match");
                        }
                        if slot_count < count {
                            fail!("bad transfer_commit: item IDs don't match");
                        }

                        if slot_count == count {
                            *slot = Item::Empty;
                        } else {
                            *slot = Item::Bulk(slot_count - count, item_id);
                        }
                    },
                    _ => {
                        fail!("bad transfer_commit: mismatched slot type (expected Bulk)");
                    },
                }
            },

            Item::Special(extra, item_id) => {
                match *slot {
                    Item::Special(slot_extra, slot_item_id) => {
                        if item_id != slot_item_id {
                            fail!("bad transfer_commit: item IDs don't match");
                        }
                        if extra != slot_extra {
                            fail!("bad transfer_commit: item extras don't match");
                        }
                        *slot = Item::Empty;
                    },
                    _ => {
                        fail!("bad transfer_commit: mismatched slot type (expected Special)");
                    },
                }
            },
        }
    }

    f.with_hooks(|h| h.on_inventory_update(iid, slot_id));
    Ok(())
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
            if acc == adjust {
                break;
            }

            match *slot {
                Item::Empty => {
                    let delta = cmp::min(u8::MAX as u16, adjust - acc) as u8;
                    *slot = Item::Bulk(delta, item_id);
                    updated_slots.push(idx as u8);
                    acc += delta as u16;
                },
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
            if acc == adjust {
                break;
            }

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
