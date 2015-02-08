var Config = require('config').Config;
var ItemDef = require('items').ItemDef;
var fromTemplate = require('util').fromTemplate;


/** @constructor */
function InventoryTracker(conn) {
    this.handler = {};
    this.conn = conn;

    var this_ = this;
    this.conn.onInventoryUpdate = function(inventory_id, updates) {
        this_._handleUpdate(inventory_id, updates);
    };
}
exports.InventoryTracker = InventoryTracker;

InventoryTracker.prototype._handleUpdate = function(inventory_id, updates) {
    var handler = this.handler[inventory_id];
    if (handler != null) {
        handler(updates);
    } else {
        console.warn('received unexpected update for inventory', inventory_id);
        this.conn.sendUnsubscribeInventory(inventory_id);
    }
};

InventoryTracker.prototype.addHandler = function(inventory_id, handler) {
    this.handler[inventory_id] = handler;
};

InventoryTracker.prototype.removeHandler = function(inventory_id) {
    delete this.handler[inventory_id];
    this.conn.sendUnsubscribeInventory(inventory_id);
};


/** @constructor */
function InventoryUI(tracker, inventory_id) {
    this.list = new ItemList();
    this.list.container.classList.add('active');

    this.container = fromTemplate('inventory', { 'item_list': this.list.container });

    this.tracker = tracker;
    this.inventory_id = inventory_id;
    this.dialog = null;

    this.on_selection_change = null;
}
exports.InventoryUI = InventoryUI;

InventoryUI.prototype._handleKeyEvent = function(down, evt) {
    if (!down) {
        return;
    }

    var binding = Config.keybindings.get()[evt.keyCode];

    var mag = evt.shiftKey ? 10 : 1;

    switch (binding) {
        case 'move_up':
            this.list.step(-1 * mag);
            break;
        case 'move_down':
            this.list.step(1 * mag);
            break;
        case 'cancel':
            this.dialog.hide();
            break;
    }
};

InventoryUI.prototype.handleOpen = function(dialog) {
    var this_ = this;
    this.dialog = dialog;
    dialog.keyboard.pushHandler(function(d, e) { return this_._handleKeyEvent(d, e); });
    this.list.track(this.tracker, this.inventory_id);
};

InventoryUI.prototype.handleClose = function(dialog) {
    this.dialog = null;
    dialog.keyboard.popHandler();
    this.tracker.removeHandler(this.inventory_id);
};

InventoryUI.prototype.enableSelect = function(last_selection, onchange) {
    this.list.default_id = last_selection;
    this.list.on_change_row = onchange;
};

InventoryUI.prototype.disableSelect = function() {
    this.list.default_id = -1;
    this.list.on_change_row = null;
}


/** @constructor */
function ItemList() {
    this.container = document.createElement('div');
    this.container.classList.add('item-list');

    this.rows = [];
    this.current_row = -1;
    this.default_id = -1;
    this.on_change_row = null;
}
exports.ItemList = ItemList;

ItemList.prototype._setCurrentRow = function(new_idx) {
    var old_idx = this.current_row;

    if (old_idx != -1) {
        this.rows[old_idx].container.classList.remove('active');
    }

    if (new_idx != -1) {
        this.rows[new_idx].container.classList.add('active');
    }

    if (this.on_change_row != null) {
        if (new_idx == -1) {
            this.on_change_row(-1);
        } else {
            this.on_change_row(this.rows[new_idx].id);
        }
    }

    this.current_row = new_idx;
};

ItemList.prototype._scrollToFocus = function() {
    if (this.rows.length == 0) {
        return;
    }

    if (this.container.scrollHeight <= this.container.clientHeight) {
        return;
    }

    var idx = this.current_row;
    if (idx < 0) {
        return;
    }

    var item_height = this.rows[0].container.clientHeight;
    var viewport_height = this.container.clientHeight;
    this.container.scrollTop = (idx + 0.5) * item_height - 0.5 * viewport_height;
};

ItemList.prototype._buildRow = function(id, qty) {
    var def = ItemDef.by_id[id];
    return new ItemRow(id, qty, def.ui_name, def.tile_x, def.tile_y);
};

ItemList.prototype.step = function(offset) {
    if (this.rows.length == 0) {
        return;
    }

    var new_row = this.current_row + offset;
    if (new_row < 0) {
        new_row = 0;
    } else if (new_row >= this.rows.length) {
        new_row = this.rows.length - 1;
    }
    this._setCurrentRow(new_row);
    this._scrollToFocus();
};

ItemList.prototype.getSelectedId = function() {
    if (this.current_row == -1) {
        return -1;
    } else {
        return this.rows[this.current_row].id;
    }
};

ItemList.prototype.track = function(tracker, inventory_id) {
    var this_ = this;
    tracker.addHandler(inventory_id, function(updates) {
        this_.update(updates);
    });
};

ItemList.prototype.update = function(updates) {
    // 'updates' contains updates sent with an InventoryUpdate message.  Each
    // one has the fields 'item_id', 'old_count', and 'new_count'.

    updates.sort(function(a, b) { return a.item_id - b.item_id; });

    // Find the ID of the currently selected item.
    var current_id = -1;
    if (this.current_row != -1) {
        current_id = this.rows[this.current_row].id;
        this._setCurrentRow(-1)
    }

    var old_rows = this.rows;
    var new_rows = [];

    var i = 0;
    var j = 0;
    var last_node = null;

    while (i < old_rows.length && j < updates.length) {
        if (updates[j].old_count == updates[j].new_count) {
            ++j;
            continue;
        }

        var old_id = old_rows[i].id;
        var update_id = updates[j].item_id;

        if (old_id < update_id) {
            // Lowest ID is an old row with no corresponding update.
            new_rows.push(old_rows[i]);
            last_node = old_rows[i].container;
            ++i;
        } else if (old_id > update_id) {
            // Lowest ID is an update that introduces a new item type.
            var new_row = this._buildRow(update_id, updates[j].new_count);
            new_rows.push(new_row);
            ++j;

            if (last_node == null) {
                this.container.insertBefore(new_row.container, this.container.firstChild);
            } else {
                this.container.insertBefore(new_row.container, last_node.nextSibling);
            }
            last_node = new_row.container;
        } else if (/* old_id == new_id && */ updates[j].new_count == 0) {
            // Lowest ID is an update that removes an existing row.
            this.container.removeChild(old_rows[i].container);
            ++i;
            ++j;
        } else /* old_id == new_id && updates[j].new_count > 0 */ {
            // Lowest ID is an update that changes the quantity of an existing
            // row.
            old_rows[i].setQuantity(updates[j].new_count);
            new_rows.push(old_rows[i]);
            last_node = old_rows[i].container;
            ++i;
            ++j;
        }
    }

    while (i < old_rows.length) {
        new_rows.push(old_rows[i]);
        ++i;
    }

    while (j < updates.length) {
        var new_row = this._buildRow(updates[j].item_id, updates[j].new_count);
        new_rows.push(new_row);
        this.container.appendChild(new_row.container);
        ++j;
    }

    this.rows = new_rows;

    // Update the current row index to point to the same item type as before,
    // or to point somewhere reasonable if that item is no longer present.
    if (current_id == -1 && new_rows.length > 0) {
        current_id = this.default_id;
    }

    if (current_id != -1) {
        var new_row = findRow(this.rows, current_id);
        if (new_row >= this.rows.length) {
            new_row = this.rows.length - 1;
        }
        this._setCurrentRow(new_row);
    } else if (new_rows.length > 0) {
        this._setCurrentRow(0);
    }
    this._scrollToFocus();
}


/** @constructor */
function ItemRow(id, qty, name, icon_x, icon_y) {
    this.container = document.createElement('div');
    this.container.classList.add('item');

    var quantityDiv = document.createElement('div');
    quantityDiv.classList.add('item-qty');
    quantityDiv.textContent = '' + qty;
    this.container.appendChild(quantityDiv);
    this.quantityDiv = quantityDiv;

    var iconDiv = document.createElement('div');
    iconDiv.classList.add('item-icon');
    iconDiv.style.backgroundPosition = '-' + icon_x + 'rem -' + icon_y + 'rem';
    this.container.appendChild(iconDiv);

    var nameDiv = document.createElement('div');
    nameDiv.classList.add('item-name');
    nameDiv.textContent = name;
    this.container.appendChild(nameDiv);

    this.id = id;
    this.qty = qty;
}
exports.ItemRow = ItemRow;

ItemRow.prototype.setQuantity = function(qty) {
    this.qty = qty;
    this.quantityDiv.textContent = '' + qty;
};


function findRow(a, id) {
    var low = 0;
    var high = a.length;

    while (low < high) {
        var mid = (low + high) >> 1;
        if (a[mid].id == id) {
            return mid;
        } else if (a[mid].id < id) {
            low = mid + 1;
        } else {
            high = mid;
        }
    }

    return low;
}

function test_findRow() {
    function run(a_id, id) {
        var a = a_id.map(function(x) { return ({ id: x }); });
        return findRow(a, id);
    }

    function check(a, id, expect) {
        var l = run(a, id);
        var r = expect;
        console.assert(l == r,
                'findRow test failure: find([' + a + '], ' + id + ' = ' + l + ', not ' + r);
    }

    check([], 99, 0);
    check([1, 3, 5], 0, 0);
    check([1, 3, 5], 1, 0);
    check([1, 3, 5], 2, 1);
    check([1, 3, 5], 3, 1);
    check([1, 3, 5], 4, 2);
    check([1, 3, 5], 5, 2);
    check([1, 3, 5], 6, 3);
}
