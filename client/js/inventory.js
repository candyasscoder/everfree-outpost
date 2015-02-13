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
    console.assert(this.inventories[inventory_id] == null,
            'overlapping subscriptions for ', inventory_id);
    var inv = new Inventory(this, inventory_id);
    this.inventories[inventory_id] = inv;
    return inv;
};

InventoryTracker.prototype.unsubscribe = function(inventory_id) {
    var deleted = delete this.inventories[inventory_id];
    console.assert(deleted,
            'tried to cancel nonexistent subscription for inventory', inventory_id);
    this.conn.sendUnsubscribeInventory(inventory_id);
};


/** @constructor */
function Inventory(owner, id) {
    this._owner = owner;
    this._id = id;
    this.onupdate = null;
    this._contents = {};
}

Inventory.prototype.getId = function() {
    return this._id;
};

Inventory.prototype.count = function(item_id) {
    return this._contents[item_id] || 0;
};

Inventory.prototype.itemIds = function() {
    return Object.getOwnPropertyNames(this._contents);
};

Inventory.prototype._update = function(updates) {
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

    if (this.onupdate != null) {
        this.onupdate(updates);
    }
};

Inventory.prototype.unsubscribe = function() {
    this._owner.unsubscribe(this._id);
};

Inventory.prototype.onUpdate = function(handler) {
    this.onupdate = chain(this.onupdate, handler);
};
