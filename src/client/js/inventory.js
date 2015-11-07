var chain = require('util/misc').chain;


var TAG_EMPTY = 0;
var TAG_BULK = 1;
var TAG_SPECIAL = 2;

exports.TAG = {
    EMPTY: TAG_EMPTY,
    BULK: TAG_BULK,
    SPECIAL: TAG_SPECIAL,
};




/** @constructor */
function InventoryTracker(conn) {
    this.server_invs = {};
    this.client_invs = {};
    this.conn = conn;

    var this_ = this;
    this.conn.onInventoryAppear = function(inventory_id, slots) {
        this_._handleAppear(inventory_id, slots);
    };
    this.conn.onInventoryUpdate = function(inventory_id, slot_idx, item) {
        this_._handleUpdate(inventory_id, slot_idx, item);
    };
    this.conn.onInventoryGone = function(inventory_id) {
        this_._handleGone(inventory_id);
    };
}
exports.InventoryTracker = InventoryTracker;

InventoryTracker.prototype.reset = function() {
    // Try to break some cycles.
    var keys = Object.getOwnPropertyNames(this.client_invs);
    for (var i = 0; i < keys.length; ++i) {
        this.client_invs[keys[i]]._handlers = null;
    }

    this.server_invs = {};
    this.client_invs = {};
};

InventoryTracker.prototype.get = function(inventory_id) {
    var new_inv = new InventoryView(this, inventory_id);
    if (this.client_invs[inventory_id] == null) {
        this.client_invs[inventory_id] = [];
    }
    this.client_invs[inventory_id].push(new_inv);
    return new_inv;
};

InventoryTracker.prototype._release = function(inventory_id, obj) {
    var arr = this.client_invs[inventory_id];
    if (arr == null) {
        console.warn('inventory: double release() (empty list)');
        return;
    }

    var found_it = false;
    for (var i = 0; i < arr.length; ++i) {
        if (arr[i] === obj) {
            arr[i] = arr[arr.length - 1];
            arr.pop();
            found_it = true;
            break;
        }
    }

    if (!found_it) {
        console.warn('inventory: double release() (not found in list)');
        return;
    }

    if (arr.length == 0) {
        delete this.client_invs[inventory_id];
    }
};

InventoryTracker.prototype.unsubscribe = function(inventory_id) {
    this.conn.sendUnsubscribeInventory(inventory_id);
    // Don't do anything else until we get the InventoryGone message.
};

InventoryTracker.prototype._getSize = function(inventory_id) {
    console.log(inventory_id);
    return this.server_invs[inventory_id].length;
};

InventoryTracker.prototype._getSlot = function(inventory_id, idx) {
    var slot = this.server_invs[inventory_id][idx];
    return {
        tag: slot.tag,
        count: slot.count,
        item_id: slot.item_id,
    };
};

InventoryTracker.prototype._countItems = function(inventory_id, item_id) {
    var inv = this.server_invs[inventory_id];
    if (inv == null) {
        return 0;
    }

    var count = 0;
    for (var i = 0; i < inv.length; ++i) {
        var item = inv[i];
        if (item.item_id != item_id) {
            continue;
        }
        if (item.tag == TAG_BULK) {
            count += item.count;
        } else if (item.tag == TAG_SPECIAL) {
            // `count` field actually stores the script ID.
            count += 1;
        }
    }
    return count;
};

InventoryTracker.prototype._getItemIds = function(inventory_id) {
    var inv = this.server_invs[inventory_id];
    if (inv == null) {
        return [];
    }

    var arr = [];
    var seen = {};
    for (var i = 0; i < inv.length; ++i) {
        var item = inv[i];
        if (item.tag == TAG_EMPTY) {
            continue;
        }
        if (seen[item.item_id]) {
            continue;
        }
        seen[item.item_id] = true;
        arr.push(item.item_id);
    }
    return arr;
};

InventoryTracker.prototype._handleAppear = function(inventory_id, slots) {
    if (this.server_invs[inventory_id] != null) {
        console.warn('server bug: got two InventoryAppear in a row', inventory_id);
        this._handleGone(inventory_id);
    }
    this.server_invs[inventory_id] = slots;

    var clients = this.client_invs[inventory_id];
    if (clients == null) {
        return;
    }
    var empty = {tag: TAG_EMPTY, count: 0, item_id: 0};
    for (var i = 0; i < clients.length; ++i) {
        for (var j = 0; j < clients[i]._handlers.length; ++j) {
            var f = clients[i]._handlers[j];
            for (var k = 0; k < slots.length; ++k) {
                if (slots[k].tag != TAG_EMPTY) {
                    f(k, empty, slots[k]);
                }
            }
        }
    }
};

InventoryTracker.prototype._handleGone = function(inventory_id) {
    var inv = this.server_invs[inventory_id];
    if (inv == null) {
        console.warn('server bug: InventoryGone without InventoryAppear', inventory_id);
        inv = [];
    }
    delete this.server_invs[inventory_id];

    // Tell clients that all slots have become empty.
    var clients = this.client_invs[inventory_id];
    if (clients == null) {
        return;
    }
    var empty = {tag: TAG_EMPTY, count: 0, item_id: 0};
    for (var i = 0; i < clients.length; ++i) {
        for (var j = 0; j < clients[i]._handlers.length; ++i) {
            var f = clients[i]._handlers[j];
            for (var k = 0; k < inv.length; ++k) {
                if (inv[k].tag != TAG_EMPTY) {
                    f(k, inv[k], empty);
                }
            }
        }
    }
};

InventoryTracker.prototype._handleUpdate = function(inventory_id, slot_idx, item) {
    var inv = this.server_invs[inventory_id];
    if (inv == null) {
        console.warn('server bug: InventoryUpdate without InventoryAppear', inventory_id);
        inv = {};
    }
    var old_item = inv[slot_idx];
    inv[slot_idx] = item;

    // Tell clients that all slots have become empty.
    var clients = this.client_invs[inventory_id];
    if (clients == null) {
        return;
    }
    for (var i = 0; i < clients.length; ++i) {
        for (var j = 0; j < clients[i]._handlers.length; ++j) {
            var f = clients[i]._handlers[j];
            f(slot_idx, old_item, item);
        }
    }
};


/** @constructor */
function InventoryView(owner, id) {
    this._owner = owner;
    this._id = id;
    this._handlers = [];
}

InventoryView.prototype.getId = function() {
    return this._id;
};

InventoryView.prototype.release = function() {
    this._owner._release(this._id, this);
};

InventoryView.prototype.clone = function() {
    return this._owner.get(this._id);
};

InventoryView.prototype.unsubscribe = function() {
    this._owner.unsubscribe(this._id);
    this.release();
};

InventoryView.prototype.count = function(item_id) {
    return this._owner._countItems(this._id, item_id);
};

InventoryView.prototype.itemIds = function() {
    return this._owner._getItemIds(this._id);
};

InventoryView.prototype.onUpdate = function(handler) {
    this._handlers.push(handler);
};

InventoryView.prototype.size = function() {
    return this._owner._getSize(this._id);
};

InventoryView.prototype.getSlot = function(i) {
    return this._owner._getSlot(this._id, i);
};
