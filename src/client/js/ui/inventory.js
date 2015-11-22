var Config = require('config').Config;
var ItemDef = require('data/items').ItemDef;
var fromTemplate = require('util/misc').fromTemplate;
var InventoryTracker = require('inventory').InventoryTracker;
var widget = require('ui/widget');
var util = require('util/misc');
var TAG = require('inventory').TAG;


/** @constructor */
function InventoryUI(dnd, inv, title) {
    this.list = new ItemGrid(inv, 6);
    this.list.dom.classList.add('active');

    var dom = fromTemplate('inventory', { 'item_list': this.list.dom });
    if (title != null) {
        dom.dataset['dialogTitle'] = title;
    }

    widget.Form.call(this, this.list, dom);
    this.onselect = null;

    var this_ = this;
    var dnd_cb = function(from_inv, from_slot, to_inv, to_slot, count) {
        if (to_inv !== from_inv) {
            return;
        }
        if (this_.ontransfer != null) {
            var id = from_inv.inv.getId();
            this_.ontransfer(id, from_slot, id, to_slot, count);
        }
    };
    this.ontransfer = null;

    this.list.registerDragSource(dnd, dnd_cb);
    this.list.registerDragTarget(dnd);
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
function ContainerUI(dnd, inv1, inv2) {
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

    var dnd_cb = function(from_inv, from_slot, to_inv, to_slot, count) {
        if (to_inv.constructor !== ItemGrid) {
            return;
        }
        if (this_.ontransfer != null) {
            this_.ontransfer(from_inv.inv.getId(), from_slot, to_inv.inv.getId(), to_slot, count);
        }
    };
    this.lists[0].registerDragSource(dnd, dnd_cb);
    this.lists[0].registerDragTarget(dnd);
    this.lists[1].registerDragSource(dnd, dnd_cb);
    this.lists[1].registerDragTarget(dnd);

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

    var from_inv_id = this.lists[fromIndex].inv.getId();
    var from_slot = sel.idx;
    var to_inv_id = this.lists[+!fromIndex].inv.getId();
    var to_slot = 255;
    var mag = evt.raw.shiftKey ? 10 : 1;

    this.ontransfer(from_inv_id, from_slot, to_inv_id, to_slot, mag);
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

    var this_ = this;
    for (var i = 0; i < size; ++i) {
        var s = new ItemSlot(this, i);
        s.update(inv.getSlot(i));
        (function(s) {
            s.dom.addEventListener('mouseenter', function(evt) {
                widget.requestFocus(this_);
                this_._setIndex(s.idx);
            });
        })(s);
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

    inv.onUpdate(function(idx, old_item, new_item) {
        this_.slots[idx].update(new_item);
    });

    this.ondragfinish = null;
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

ItemGrid.prototype.registerDragSource = function(dnd, callback) {
    for (var i = 0; i < this.slots.length; ++i) {
        dnd.registerSource(this.slots[i]);
    }
    this.ondragfinish = function(source_slot, target_slot, data) {
        callback(source_slot.owner, source_slot.idx,
                 target_slot.owner, target_slot.idx,
                 data.count);
    };
};

ItemGrid.prototype.registerDragTarget = function(dnd) {
    for (var i = 0; i < this.slots.length; ++i) {
        dnd.registerTarget(this.slots[i]);
    }
};


/** @constructor */
function ItemSlot(owner, idx, info) {
    var parts = util.templateParts('item-slot');
    parts['qty'].textContent = '';
    parts['icon'].style.backgroundPosition = '-0rem -0rem';

    widget.Element.call(this, parts['top']);

    this.qty_part = parts['qty'];
    this.icon_part = parts['icon'];

    this.owner = owner;
    this.idx = idx;

    this.tag = TAG.EMPTY;
    this.id = 0;
    this.qty = 0;

    this.dragging = false;

    if (info != null) {
        this.update(info);
    }
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

    if (this.dom.classList.contains('drag-source') && !this.dragging) {
        this.dom.classList.remove('drag-source');
    }
};

ItemSlot.prototype.ondragstart = function(evt) {
    if (this.tag == TAG.EMPTY) {
        return null;
    }

    var icon = this.dom.cloneNode(true);
    this.dom.classList.add('drag-source');
    this.dragging = true;
    return {
        count: this.qty,
        icon: icon,
    };
};

ItemSlot.prototype.ondragfinish = function(target, data) {
    this.dragging = false;
    if (this.owner.ondragfinish != null) {
        this.owner.ondragfinish(this, target, data);
    }
};

ItemSlot.prototype.ondragcancel = function(data) {
    this.dragging = false;
    this.dom.classList.remove('drag-source');
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
