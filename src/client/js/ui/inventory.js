var Config = require('config').Config;
var ItemDef = require('data/items').ItemDef;
var fromTemplate = require('util/misc').fromTemplate;
var InventoryTracker = require('inventory').InventoryTracker;
var widget = require('ui/widget');
var util = require('util/misc');
var TAG = require('inventory').TAG;


/** @constructor */
function InventoryUI(inv, title) {
    this.list = new ItemGrid(inv, 6);
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

InventoryUI.prototype.enableSelect = function(last_item_id, onselect) {
    this.onselect = onselect;
    this.list.selectItem(last_item_id);
};


/** @constructor */
function ContainerUI(inv1, inv2) {
    this.lists = [new ItemGrid(inv1, 6), new ItemGrid(inv2, 6)];

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

    // TODO: transfer by slot index, not by item_id
    var item_id = sel.id;
    var from_inv_id = this.lists[fromIndex].inv.getId();
    var to_inv_id = this.lists[+!fromIndex].inv.getId();
    var mag = evt.raw.shiftKey ? 10 : 1;

    this.ontransfer(from_inv_id, to_inv_id, item_id, mag);
};


/** @constructor */
function ItemGrid(inv, cols) {
    this.inv = inv;
    var size = inv.size();
    this.cols = cols;
    this.rows = ((size + cols - 1) / cols)|0;

    this.slots = new Array(size);
    this.dom = util.element('div', ['item-grid', 'g-col']);

    var row_doms = new Array(this.rows);
    for (var i = 0; i < this.rows; ++i) {
        row_doms[i] = util.element('div', ['g-row'], this.dom);
    }

    for (var i = 0; i < size; ++i) {
        var s = new ItemSlot(i);
        s.update(inv.getSlot(i));
        this.slots[i] = s;

        var row_dom = row_doms[(i / this.cols)|0];
        row_dom.appendChild(s.dom);
        if (i % cols == cols - 1) {
            this.dom.appendChild(util.element('br'));
        }
    }

    this.x = 0;
    this.y = 0;
    this.slots[0].dom.classList.add('active');

    var this_ = this;
    inv.onUpdate(function(idx, old_item, new_item) {
        this_.slots[idx].update(new_item);
    });
}
ItemGrid.prototype = Object.create(widget.Element.prototype);
ItemGrid.prototype.constructor = ItemGrid;
exports.ItemGrid = ItemGrid;

ItemGrid.prototype.onkey = function(evt) {
    var mag = evt.shiftKey ? 10 : 1;

    var new_x = this.x;
    var new_y = this.y;
    switch (evt.uiKeyName()) {
        case 'move_up': new_y -= mag; break;
        case 'move_down': new_y += mag; break;
        case 'move_left': new_x -= mag; break;
        case 'move_right': new_x += mag; break;
        default:
            return false;
    }

    if (new_x < 0) {
        new_x = 0;
    } else if (new_x >= this.cols) {
        new_x = this.cols - 1;
    }
    if (new_y < 0) {
        new_y = 0;
    } else if (new_y >= this.rows) {
        new_y = this.rows - 1;
    }

    if (new_x == this.x && new_y == this.y) {
        // Consider the event unhandled.  This lets the player move between
        // multiple grids in a list, by moving past the edge of one to get to
        // the next.
        return false;
    }
    if (evt.down) {
        this._setPos(new_x, new_y);
    }
    return true;
};

ItemGrid.prototype._setPos = function(x, y) {
    this.selection().dom.classList.remove('active');

    this.x = x;
    this.y = y;

    this.selection().dom.classList.add('active');
};

ItemGrid.prototype._setIndex = function(idx) {
    this._setPos(idx % this.cols, (idx / this.cols)|0);
};

ItemGrid.prototype._getIndex = function() {
    var idx = this.y * this.cols + this.x;
    if (idx >= this.inv.size()) {
        idx = this.inv.size() - 1;
    }
    return idx;
};

ItemGrid.prototype.selection = function() {
    return this.slots[this._getIndex()];
};

ItemGrid.prototype.selectItem = function(item_id) {
    for (var i = 0; i < this.inv.size(); ++i) {
        var info = this.inv.getSlot(i);
        if (info.item_id == item_id) {
            this._setIndex(i);
            break;
        }
    }
};


/** @constructor */
function ItemSlot(idx, info) {
    var parts = util.templateParts('item-slot');
    parts['qty'].textContent = '';
    parts['icon'].style.backgroundPosition = '-0rem -0rem';

    widget.Element.call(this, parts['top']);

    this.qty_part = parts['qty'];
    this.icon_part = parts['icon'];

    this.idx = idx;
    this.tag = TAG.EMPTY;
    this.id = 0;
    this.qty = 0;

    if (info != null) {
        this.update(info);
    }

    window.DND.registerSource(this);
}
ItemSlot.prototype = Object.create(widget.Element.prototype);
ItemSlot.prototype.constructor = ItemSlot;
exports.ItemSlot = ItemSlot;

ItemSlot.prototype.update = function(info) {
    this.tag = info.tag;
    this.id = info.item_id;
    this.qty = info.count;

    var new_qty_str = '';
    if (info.tag == TAG.EMPTY || info.tag == TAG.SPECIAL) {
        // Leave qty blank
    } else if (info.tag == TAG.BULK) {
        new_qty_str = '' + this.qty;
    } else {
        console.assert(false, 'bad tag:', info.tag);
    }

    var def = ItemDef.by_id[this.id];
    this.qty_part.textContent = new_qty_str;
    this.icon_part.style.backgroundPosition = '-' + def.tile_x + 'rem -' + def.tile_y + 'rem';
};

ItemSlot.prototype.ondragstart = function(evt) {
    console.log('ondragstart', evt);
    return {
        'inv': this.inv,
        'slot': this.idx,
        'icon': this.dom.cloneNode(true),
    };
};


// Still used for recipe input/output display
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
