var Config = require('config').Config;
var ItemDef = require('items').ItemDef;
var SelectionList = require('sortedlist').SelectionList;
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
    this.list.select(last_selection);
    this.list.on_change_row = onchange;
};

InventoryUI.prototype.disableSelect = function() {
    this.list.on_change_row = null;
}


/** @constructor */
function ContainerUI(tracker, inventory1_id, inventory2_id) {
    this.lists = [new ItemList(), new ItemList()];

    this.container = fromTemplate('container', {
        'item_list1': this.lists[0].container,
        'item_list2': this.lists[1].container,
    });

    this.tracker = tracker;
    this.inventory_ids = [inventory1_id, inventory2_id];
    this.dialog = null;

    this.active = 0;
    this.lists[0].container.classList.add('active');

    this.on_transfer = null;
}
exports.ContainerUI = ContainerUI;

ContainerUI.prototype._activate = function(which) {
    this.lists[this.active].container.classList.remove('active');
    this.active = which;
    this.lists[this.active].container.classList.add('active');
};

ContainerUI.prototype._handleKeyEvent = function(down, evt) {
    if (!down) {
        return;
    }

    var binding = Config.keybindings.get()[evt.keyCode];

    var mag = evt.shiftKey ? 10 : 1;

    switch (binding) {
        case 'move_up':
            this.lists[this.active].step(-1 * mag);
            break;
        case 'move_down':
            this.lists[this.active].step(1 * mag);
            break;

        case 'move_left':
            if (this.active == 1) {
                this._activate(0);
            }
            break;
        case 'move_right':
            if (this.active == 0) {
                this._activate(1);
            }
            break;

        case 'interact':
            if (this.on_transfer != null) {
                var item_id = this.lists[this.active].selectedItem();
                this.on_transfer(this.active, +!this.active, item_id, mag);
            }
            break;

        case 'cancel':
            this.dialog.hide();
            break;
    }
};

ContainerUI.prototype.handleOpen = function(dialog) {
    var this_ = this;
    this.dialog = dialog;
    dialog.keyboard.pushHandler(function(d, e) { return this_._handleKeyEvent(d, e); });
    this.lists[0].track(this.tracker, this.inventory_ids[0]);
    this.lists[1].track(this.tracker, this.inventory_ids[1]);
};

ContainerUI.prototype.handleClose = function(dialog) {
    this.dialog = null;
    dialog.keyboard.popHandler();
    this.tracker.removeHandler(this.inventory_ids[0]);
    this.tracker.removeHandler(this.inventory_ids[0]);
};


/** @constructor */
function ItemList() {
    this.list = new SelectionList('item-list');
    this.container = this.list.container;

    this.on_change_row = null;

    var this_ = this;
    this.list.onchange = function(row) {
        if (row == null) {
            if (this_.on_change_row != null) {
                this_.on_change_row(-1);
            }
            return;
        }

        // If the selection changed because the selected item was removed,
        // don't switch back to that item even if it reappears.
        this_.list.select(row.id);

        if (this_.on_change_row != null) {
            this_.on_change_row(row.id);
        }

        this_._scrollToSelection();
    };
}
exports.ItemList = ItemList;

ItemList.prototype._scrollToSelection = function() {
    var sel = this.list.selection();
    if (sel == null) {
        return;
    }

    var item_bounds = sel.container.getBoundingClientRect();
    var parent_bounds = this.container.getBoundingClientRect();
    var target_top = parent_bounds.top + parent_bounds.height / 2 - item_bounds.height / 2;
    // Adjust scrollTop to move 'item_bounds.top' to 'target_top'.
    var delta = target_top - item_bounds.top;
    this.container.scrollTop += delta;
};

ItemList.prototype.select = function(id) {
    this.list.select(id);
};

ItemList.prototype.step = function(offset) {
    this.list.step(offset);
};

ItemList.prototype.track = function(tracker, inventory_id) {
    var this_ = this;
    tracker.addHandler(inventory_id, function(updates) {
        this_.update(updates);
    });
};

ItemList.prototype.update = function(updates) {
    this.list.update(updates, function(up, row) {
        if (up.new_count == 0) {
            return null;
        } else if (up.old_count == 0) {
            var id = up.id;
            var qty = up.new_count;
            var def = ItemDef.by_id[id];
            return new ItemRow(id, qty, def.ui_name, def.tile_x, def.tile_y);
        } else {
            row.setQuantity(up.new_count);
            return row;
        }
    });
};

ItemList.prototype.selectedItem = function() {
    var sel = this.list.selection();
    if (sel == null) {
        return -1;
    } else {
        return sel.id;
    }
};


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
