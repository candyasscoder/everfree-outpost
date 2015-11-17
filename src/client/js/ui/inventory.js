var Config = require('config').Config;
var ItemDef = require('data/items').ItemDef;
var fromTemplate = require('util/misc').fromTemplate;
var InventoryTracker = require('inventory').InventoryTracker;
var widget = require('ui/widget');
var util = require('util/misc');


/** @constructor */
function InventoryUI(inv, title) {
    this.list = new ItemList(inv);
    this.list.dom.classList.add('active');

    var dom = fromTemplate('inventory', { 'item_list': this.list.dom });
    if (title != null) {
        dom.dataset['dialogTitle'] = title;
    }

    widget.Form.call(this, this.list, dom);
    this.onselect = null;
}
InventoryUI.prototype = Object.create(widget.Form.prototype);
InventoryUI.prototype.constructor = InventoryUI;
exports.InventoryUI = InventoryUI;

InventoryUI.prototype.onkey = function(evt) {
    if (widget.Form.prototype.onkey.call(this, evt)) {
        return true;
    }

    var binding = evt.uiKeyName();
    if (binding != null && binding.startsWith('set_hotbar_')) {
        var sel = this.list.selection();
        if (sel != null && this.onselect != null) {
            var idx = +binding.substring(11) - 1;
            this.onselect(idx, sel.id);
        }
        return true;
    }
};

InventoryUI.prototype.enableSelect = function(last_selection, onselect) {
    this.onselect = onselect;
    this.list.select(last_selection);
};


/** @constructor */
function ContainerUI(inv1, inv2) {
    this.lists = [new ItemList(inv1), new ItemList(inv2)];

    var dom = fromTemplate('container', {
        'item_list1': this.lists[0].dom,
        'item_list2': this.lists[1].dom,
    });
    var container = new widget.SimpleList(dom, this.lists, ['move_left', 'move_right']);

    widget.Form.call(this, container);

    var this_ = this;
    widget.hookKey(this.lists[0], 'select', function(evt) { this_._transfer(evt, 0) });
    widget.hookKey(this.lists[1], 'select', function(evt) { this_._transfer(evt, 1) });
    this.ontransfer = null;
}
ContainerUI.prototype = Object.create(widget.Form.prototype);
ContainerUI.prototype.constructor = ContainerUI;
exports.ContainerUI = ContainerUI;

ContainerUI.prototype._transfer = function(evt, fromIndex) {
    if (!evt.down) {
        return;
    }
    if (this.ontransfer == null) {
        return;
    }

    var sel = this.lists[fromIndex].selection();
    if (sel == null) {
        return;
    }

    var item_id = sel.id;
    var from_inv_id = this.lists[fromIndex].inventory_id;
    var to_inv_id = this.lists[+!fromIndex].inventory_id;
    var mag = evt.raw.shiftKey ? 10 : 1;

    this.ontransfer(from_inv_id, to_inv_id, item_id, mag);
};


/** @constructor */
function ItemList(inv) {
    widget.DynamicList.call(this, util.element('div', ['item-list', 'g-scrollpane', 'g-list']));

    this.inventory_id = inv.getId();

    /*
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
    */

    var item_ids = inv.itemIds();
    for (var i = 0; i < item_ids.length; ++i) {
        var id = item_ids[i];
        this.updateItems(-1,
                {tag: -1, count: 0, item_id: id},
                {tag: -1, count: inv.count(id), item_id: id});
    }

    var this_ = this;
    inv.onUpdate(function(idx, old_item, new_item) {
        this_.updateItems(idx, old_item, new_item);
    });
}
ItemList.prototype = Object.create(widget.DynamicList.prototype);
ItemList.prototype.constructor = ItemList;
exports.ItemList = ItemList;

ItemList.prototype.updateItems = function(idx, old_item, new_item) {
    var updates;
    // TODO: not correct for TAG_SPECIAL
    if (old_item.item_id != new_item.item_id) {
        updates = [
            {id: old_item.item_id, delta: -old_item.count},
            {id: new_item.item_id, delta: new_item.count},
        ];
    } else {
        updates = [
            {id: new_item.item_id, delta: new_item.count - old_item.count},
        ];
    }

    this.update(updates, function(up, row) {
        var old_count = row == null ? 0 : row.qty;
        var new_count = old_count + up.delta;

        if (new_count == 0) {
            return null;
        } else if (old_count == 0) {
            var id = up.id;
            var qty = new_count;
            var def = ItemDef.by_id[id];
            return new ItemRow(id, qty, def.ui_name, def.tile_x, def.tile_y);
        } else {
            row.setQuantity(new_count);
            return row;
        }
    });
};


/** @constructor */
function ItemRow(id, qty, name, icon_x, icon_y) {
    var parts = util.templateParts('item-row');
    parts['qty'].textContent = '' + qty;
    parts['icon'].style.backgroundPosition = '-' + icon_x + 'rem -' + icon_y + 'rem';
    parts['name'].textContent = name;

    widget.Element.call(this, parts['top']);

    this.id = id;
    this.qty = qty;
    this.quantityDiv = parts['qty'];
}
ItemRow.prototype = Object.create(widget.Element.prototype);
ItemRow.prototype.constructor = ItemRow;
exports.ItemRow = ItemRow;

ItemRow.prototype.setQuantity = function(qty) {
    this.qty = qty;
    this.quantityDiv.textContent = '' + qty;
};
