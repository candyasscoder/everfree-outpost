var util = require('util/misc');
var ItemDef = require('data/items').ItemDef;


// General-purpose item/label pair.
/** @constructor */
function ItemBox(cls, qty) {
    this.dom = util.element('div', [cls]);
    this.iconDiv = util.element('div', ['item-icon'], this.dom);
    if (qty) {
        this.qtyDiv = util.element('div', ['item-qty'], this.dom);
    } else {
        this.qtyDiv = null;
    }
}
exports.ItemBox = ItemBox;

ItemBox.prototype.setItem = function(id) {
    if (id == -1) {
        this.iconDiv.style.backgroundPosition = '-0rem -0rem';
        return;
    }
    var item = ItemDef.by_id[id];
    this.iconDiv.style.backgroundPosition = '-' + item.tile_x + 'rem -' + item.tile_y + 'rem';
};

ItemBox.prototype.setQuantity = function(qty) {
    this.qtyDiv.textContent = qty;
};

ItemBox.prototype.setDisabled = function(disabled) {
    // TODO: find a way to mark things as disabled.  filter: grayscale doesn't
    // seem to apply to background images.
};


/** @constructor */
function ActiveItems() {
    this.dom = util.element('div', ['active-items']);
    this.ability = new ItemBox('item-box', false);
    this.item = new ItemBox('item-box', true);
    this.dom.appendChild(this.ability.dom);
    this.dom.appendChild(this.item.dom);

    this._abilityId = -1;
    this._itemId = -1;

    this._itemInv = null;
    this._abilityInv = null;
}
exports.ActiveItems = ActiveItems;

ActiveItems.prototype.getAbility = function() {
    return this._abilityId;
};

ActiveItems.prototype.getItem = function() {
    return this._itemId;
};

ActiveItems.prototype.setAbility = function(id) {
    this._abilityId = id;
    this.ability.setItem(id);
    this.ability.setDisabled(this._abilityInv != null && this._abilityInv.count(id) == 0);
};

ActiveItems.prototype.setItem = function(id) {
    this._itemId = id;
    this.item.setItem(id);
    if (this._itemInv != null) {
        this.item.setQuantity('' + this._itemInv.count(id));
    } else {
        this.item.setQuantity('');
    }
};

ActiveItems.prototype.attachAbilities = function(inv) {
    this._abilityInv = inv;

    var this_ = this;
    inv.onUpdate(function(updates) {
        console.log('update abilities', this_._abilityId,
                inv.count(this_._abilityId));
        if (this_._abilityId != -1) {
            this_.ability.setDisabled(inv.count(this_._abilityId) == 0);
        }
    });
};

ActiveItems.prototype.attachItems = function(inv) {
    this._itemInv = inv;

    var this_ = this;
    inv.onUpdate(function(updates) {
        if (this_._itemId != -1) {
            this_.item.setQuantity('' + inv.count(this_._itemId));
        }
    });
};
