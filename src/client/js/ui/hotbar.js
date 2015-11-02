var Config = require('config').Config;
var util = require('util/misc');
var ItemDef = require('data/items').ItemDef;


// General-purpose item/label pair.
/** @constructor */
function ItemBox(has_qty) {
    var parts = util.templateParts('item-box');
    this.iconDiv = parts['icon'];

    if (has_qty) {
        this.qtyDiv = parts['qty'];
    } else {
        parts['top'].removeChild(parts['qty']);
    }

    this.dom = parts['top'];
}

ItemBox.prototype.setItem = function(id) {
    if (id == -1) {
        this.iconDiv.style.backgroundPosition = '-0rem -0rem';
        return;
    }
    var item = ItemDef.by_id[id];
    this.iconDiv.style.backgroundPosition = '-' + item.tile_x + 'rem -' + item.tile_y + 'rem';
};

ItemBox.prototype.setQuantity = function(qty) {
    if (qty != -1) {
        this.qtyDiv.textContent = '' + qty;
    } else {
        this.qtyDiv.textContent = '';
    }
};

ItemBox.prototype.setDisabled = function(disabled) {
    // TODO: find a way to mark things as disabled.  filter: grayscale doesn't
    // seem to apply to background images.
};


/** @constructor */
function Hotbar() {
    this.dom = util.fromTemplate('hotbar', {});
    this.boxes = new Array(9);
    for (var i = 0; i < this.boxes.length; ++i) {
        this.boxes[i] = new ItemBox(true);
        this.dom.appendChild(this.boxes[i].dom);
    }

    this.item_ids = new Array(9);
    this.is_item = new Array(9);
    for (var i = 0; i < 9; ++i) {
        this.item_ids[i] = -1;
        // Suppress quantity display for unused slots.
        this.is_item[i] = false;
    }

    this.active_item = -1;
    this.active_ability = -1;
    
    this.item_inv = null;
    this.ability_inv = null;
}
exports.Hotbar = Hotbar;

Hotbar.prototype._setSlotInfo = function(idx, item_id, is_item) {
    if (is_item && this.active_ability == idx) {
        this._setActiveAbility(-1);
    }
    if (!is_item && this.active_item == idx) {
        this._setActiveItem(-1);
    }

    this.item_ids[idx] = item_id;
    this.is_item[idx] = is_item;

    var box = this.boxes[idx];
    box.setItem(item_id);
    if (is_item) {
        var qty = this.item_inv != null ? this.item_inv.count(item_id) : 0;
        box.setQuantity(qty);
    } else {
        box.setQuantity(-1);
    }
};

Hotbar.prototype.init = function() {
    var cfg = Config.hotbar.get();
    var names = cfg['names'] || [];
    var is_item_arr = cfg['is_item'] || [];

    for (var i = 0; i < names.length && i < this.item_ids.length; ++i) {
        var item = ItemDef.by_name[names[i]];
        if (item == null) {
            continue;
        }

        this._setSlotInfo(i, item.id, is_item_arr[i]);
    }

    if (cfg['active_item'] != null) {
        this._setActiveItem(cfg['active_item']);
    }
    if (cfg['active_ability'] != null) {
        this._setActiveAbility(cfg['active_ability']);
    }
};

Hotbar.prototype.setSlot = function(idx, item_id, is_item) {
    if (idx < 0 || idx >= this.item_ids.length) {
        return;
    }

    var cfg = Config.hotbar.get();
    cfg['names'][idx] = ItemDef.by_id[item_id].name;
    cfg['is_item'][idx] = is_item;
    Config.hotbar.save();

    this._setSlotInfo(idx, item_id, is_item);
};

Hotbar.prototype.selectSlot = function(idx) {
    if (idx < 0 || idx >= this.item_ids.length) {
        return;
    }
    if (this.item_ids[idx] == -1) {
        return;
    }

    if (this.is_item[idx]) {
        this._setActiveItem(idx);
    } else {
        this._setActiveAbility(idx);
    }
};

Hotbar.prototype._setActiveAbility = function(idx) {
    // Valid indices are -1 .. len-1.  -1 indicates "no selection".
    if (idx < -1 || idx >= this.item_ids.length || this.is_item[idx]) {
        return;
    }

    if (this.active_ability != -1) {
        this.boxes[this.active_ability].dom.classList.remove('active-ability');
    }
    this.active_ability = idx;
    if (this.active_ability != -1) {
        this.boxes[this.active_ability].dom.classList.add('active-ability');
    }

    Config.hotbar.get()['active_ability'] = idx;
    Config.hotbar.save();
};

Hotbar.prototype._setActiveItem = function(idx) {
    // Valid indices are -1 .. len-1.  -1 indicates "no selection".
    if (idx < -1 || idx >= this.item_ids.length || !this.is_item[idx]) {
        return;
    }

    if (this.active_item != -1) {
        this.boxes[this.active_item].dom.classList.remove('active-item');
    }
    this.active_item = idx;
    if (this.active_item != -1) {
        this.boxes[this.active_item].dom.classList.add('active-item');
    }

    Config.hotbar.get()['active_item'] = idx;
    Config.hotbar.save();
};

Hotbar.prototype.getAbility = function() {
    if (this.active_ability != -1) {
        return this.item_ids[this.active_ability];
    } else {
        return -1;
    }
};

Hotbar.prototype.getItem = function() {
    if (this.active_item != -1) {
        return this.item_ids[this.active_item];
    } else {
        return -1;
    }
};

Hotbar.prototype.attachAbilities = function(inv) {
    if (this.ability_inv != null) {
        this.ability_inv.release();
    }
    this.ability_inv = inv;
    // Not actually used for anything.
    // TODO: gray out abilities when they become unusable.
};

Hotbar.prototype._updateItems = function() {
    for (var i = 0; i < this.item_ids.length; ++i) {
        if (!this.is_item[i]) {
            continue;
        }

        this.boxes[i].setQuantity(this.item_inv.count(this.item_ids[i]));
    }
};

Hotbar.prototype.attachItems = function(inv) {
    if (this.item_inv != null) {
        this.item_inv.release();
    }
    this.item_inv = inv;

    var this_ = this;
    inv.onUpdate(function(idx, old_item, new_item) {
        // TODO: might be slow (O(N^2)) at startup time
        this_._updateItems();
    });
};
