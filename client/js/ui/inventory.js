var Config = require('config').Config;
var ItemDef = require('data/items').ItemDef;
var SelectionList = require('ui/sortedlist').SelectionList;
var fromTemplate = require('util/misc').fromTemplate;
var InventoryTracker = require('inventory').InventoryTracker;
var chain = require('util/misc').chain;


/** @constructor */
function InventoryUI(inv) {
    this.list = new ItemList(inv);
    this.list.container.classList.add('active');

    this.container = fromTemplate('inventory', { 'item_list': this.list.container });

    this.dialog = null;

    this.onclose = null;
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
};

InventoryUI.prototype.handleClose = function(dialog) {
    this.dialog = null;
    dialog.keyboard.popHandler();

    if (this.onclose != null) {
        this.onclose();
    }
};

InventoryUI.prototype.enableSelect = function(last_selection, onchange) {
    this.list.select(last_selection);
    this.list.onchange = onchange;
};

InventoryUI.prototype.disableSelect = function() {
    this.list.onchange = null;
}


/** @constructor */
function ContainerUI(inv1, inv2) {
    this.lists = [new ItemList(inv1), new ItemList(inv2)];

    this.container = fromTemplate('container', {
        'item_list1': this.lists[0].container,
        'item_list2': this.lists[1].container,
    });

    this.dialog = null;

    this.active = 0;
    this.lists[0].container.classList.add('active');

    this.ontransfer = null;
    this.onclose = null;
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
            if (this.ontransfer != null) {
                var item_id = this.lists[this.active].selectedItem();
                var from_inv_id = this.lists[this.active].inventory_id;
                var to_inv_id = this.lists[+!this.active].inventory_id;
                this.ontransfer(from_inv_id, to_inv_id, item_id, mag);
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
};

ContainerUI.prototype.handleClose = function(dialog) {
    this.dialog = null;
    dialog.keyboard.popHandler();

    if (this.onclose != null) {
        this.onclose();
    }
};


/** @constructor */
function ItemList(inv) {
    this.inventory_id = inv.getId();
    this.list = new SelectionList('item-list');
    this.container = this.list.container;

    this.onchange = null;

    var this_ = this;
    this.list.onchange = function(row) {
        if (row == null) {
            if (this_.onchange != null) {
                this_.onchange(-1);
            }
            return;
        }

        // If the selection changed because the selected item was removed,
        // don't switch back to that item even if it reappears.
        this_.list.select(row.id);

        if (this_.onchange != null) {
            this_.onchange(row.id);
        }

        this_._scrollToSelection();
    };

    var init = inv.itemIds().map(function(id) {
        return { id: id, old_count: 0, new_count: inv.count(id) };
    });
    this.update(init);

    inv.onUpdate(function(updates) {
        this_.update(updates);
    });
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
