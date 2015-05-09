var Config = require('config').Config;
var ItemDef = require('data/items').ItemDef;
var SelectionList = require('ui/sortedlist').SelectionList;
var fromTemplate = require('util/misc').fromTemplate;
var InventoryTracker = require('inventory').InventoryTracker;
var chain = require('util/misc').chain;
var widget = require('ui/widget');


/** @constructor */
function InventoryUI(inv, title) {
    this.list = new ItemList(inv);
    this.list.dom.classList.add('active');

    this.dom = fromTemplate('inventory', { 'item_list': this.list.dom });
    this.keys = this.list.keys;

    if (title != null) {
        this.dom.getElementsByClassName('title')[0].textContent = title;
    }

    this.dialog = null;

    this.onclose = null;
}
exports.InventoryUI = InventoryUI;

InventoryUI.prototype.handleOpen = function(dialog) {
    this.dialog = dialog;
};

InventoryUI.prototype.handleClose = function(dialog) {
    if (this.onclose != null) {
        this.onclose();
    }
};

InventoryUI.prototype.enableSelect = function(last_selection, onchange) {
    this.list.select(last_selection);
    this.list.onchange = onchange;
    if (onchange != null) {
        onchange(this.list.selectedItem());
    }
};

InventoryUI.prototype.disableSelect = function() {
    this.list.onchange = null;
}


/** @constructor */
function ContainerUI(inv1, inv2) {
    this.lists = [new ItemList(inv1), new ItemList(inv2)];

    this.dom = fromTemplate('container', {
        'item_list1': this.lists[0].dom,
        'item_list2': this.lists[1].dom,
    });

    var this_ = this;
    this.focus = new widget.FocusTracker(this.lists, ['move_left', 'move_right']);
    this.keys = new widget.ActionKeyHandler(
            'select',
            function(evt) { this_._transfer(evt.shiftKey ? 10 : 1); },
            this.focus);

    this.dialog = null;

    this.ontransfer = null;
    this.onclose = null;
}
exports.ContainerUI = ContainerUI;

ContainerUI.prototype._transfer = function(mag) {
    if (this.ontransfer != null) {
        var active = this.focus.selectedIndex();
        var item_id = this.lists[active].selectedItem();
        var from_inv_id = this.lists[active].inventory_id;
        var to_inv_id = this.lists[+!active].inventory_id;
        this.ontransfer(from_inv_id, to_inv_id, item_id, mag);
    }
};

ContainerUI.prototype.handleOpen = function(dialog) {
    this.dialog = dialog;
};

ContainerUI.prototype.handleClose = function(dialog) {
    if (this.onclose != null) {
        this.onclose();
    }
};


/** @constructor */
function ItemList(inv) {
    this.inventory_id = inv.getId();
    this.list = new SelectionList('item-list');
    this.dom = this.list.dom;
    this.keys = this.list.keys;

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

    var item_bounds = sel.dom.getBoundingClientRect();
    var parent_bounds = this.dom.getBoundingClientRect();
    var target_top = parent_bounds.top + parent_bounds.height / 2 - item_bounds.height / 2;
    // Adjust scrollTop to move 'item_bounds.top' to 'target_top'.
    var delta = target_top - item_bounds.top;
    // Use -= instead of += because INCREASING scrollTop causes item_bounds.top
    // to DECREASE.
    this.dom.scrollTop -= delta;
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
    this.dom = document.createElement('div');
    this.dom.classList.add('item');
    this.keys = widget.NULL_KEY_HANDLER;

    var quantityDiv = document.createElement('div');
    quantityDiv.classList.add('item-qty');
    quantityDiv.textContent = '' + qty;
    this.dom.appendChild(quantityDiv);
    this.quantityDiv = quantityDiv;

    var iconDiv = document.createElement('div');
    iconDiv.classList.add('item-icon');
    iconDiv.style.backgroundPosition = '-' + icon_x + 'rem -' + icon_y + 'rem';
    this.dom.appendChild(iconDiv);

    var nameDiv = document.createElement('div');
    nameDiv.classList.add('item-name');
    nameDiv.textContent = name;
    this.dom.appendChild(nameDiv);

    this.id = id;
    this.qty = qty;
}
exports.ItemRow = ItemRow;

ItemRow.prototype.setQuantity = function(qty) {
    this.qty = qty;
    this.quantityDiv.textContent = '' + qty;
};
