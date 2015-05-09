var chain = require('util/misc').chain;


/** @constructor */
function InventoryTracker(conn) {
    this.inventories = {};
    this.conn = conn;

    var this_ = this;
    this.conn.onInventoryUpdate = function(inventory_id, updates) {
        this_._handleUpdate(inventory_id, updates);
    };
}
exports.InventoryTracker = InventoryTracker;

InventoryTracker.prototype._handleUpdate = function(inventory_id, updates) {
    var inv = this.inventories[inventory_id];
    if (inv != null) {
        inv._update(updates);
    } else {
        console.assert(false, 'received unexpected update for inventory', inventory_id);
    }
};

InventoryTracker.prototype.subscribe = function(inventory_id) {
    if (this.inventories[inventory_id] != null) {
        ++this.inventories[inventory_id]._ref_count;
    } else {
        this.inventories[inventory_id] = new InventoryData(this, inventory_id);
    }
    return new Inventory(this, inventory_id);
};

InventoryTracker.prototype._unsubscribe = function(inventory_id) {
    var inv = this.inventories[inventory_id];
    if (inv == null) {
        console.warn('tried to cancel nonexistent subscription for inventory', inventory_id);
    }
    // UnsubscribeInventory only decrements the server-side subscription
    // refcount.  It doesn't actually end the subscription until the count hits
    // zero.
    this.conn.sendUnsubscribeInventory(inventory_id);

    --inv._ref_count;
    if (inv._ref_count == 0) {
        delete this.inventories[inventory_id];
    }
};


/** @constructor */
function InventoryData(owner, id) {
    this._owner = owner;
    this._id = id;
    this._contents = {};
    this._handlers = [];
    this._ref_count = 1;
}

InventoryData.prototype._update = function(updates) {
    for (var i = 0; i < updates.length; ++i) {
        var update = updates[i];
        if (update.new_count == 0) {
            if (update.old_count > 0) {
                delete this._contents[update.id];
            }
        } else {
            this._contents[update.id] = update.new_count;
        }
    }

    for (var i = 0; i < this._handlers.length; ++i) {
        this._handlers[i](updates);
    }
};

InventoryData.prototype._addHandler = function(handler) {
    this._handlers.push(handler);
};

InventoryData.prototype._removeHandler = function(handler) {
    var idx = -1;
    for (var i = 0; i < this._handlers.length; ++i) {
        if (this._handlers[i] === handler) {
            idx = i;
            break;
        }
    }

    if (idx == -1) {
        console.warn('tried to remove unregistered handler', handler);
        return;
    }

    this._handlers[idx] = this._handlers[this._handlers.length - 1];
    this._handlers.pop();
};


/** @constructor */
function Inventory(owner, id, hold_ref) {
    this._owner = owner;
    this._id = id;
    this._handlers = [];
    this._holds_ref = hold_ref != null ? hold_ref : true;
}

Inventory.prototype._data = function() {
    return this._owner.inventories[this._id];
};

Inventory.prototype.getId = function() {
    return this._id;
};

Inventory.prototype.getRefCount = function() {
    return this._data()._ref_count;
};

Inventory.prototype.count = function(item_id) {
    return this._data()._contents[item_id] || 0;
};

Inventory.prototype.itemIds = function() {
    var ids = Object.getOwnPropertyNames(this._data()._contents);
    // Convert all to numbers.
    for (var i = 0; i < ids.length; ++i) {
        ids[i] = +ids[i];
    }
    return ids;
};

Inventory.prototype.onUpdate = function(handler) {
    this._handlers.push(handler);
    this._data()._addHandler(handler);
};

Inventory.prototype.unsubscribe = function() {
    for (var i = 0; i < this._handlers.length; ++i) {
        this._data()._removeHandler(this._handlers[i]);
    }
    this._handlers = [];
    if (this._holds_ref) {
        this._owner._unsubscribe(this._id);
    }
};

Inventory.prototype.clone = function() {
    return new Inventory(this._owner, this._id, false);
};
