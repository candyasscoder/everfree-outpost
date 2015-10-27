var util = require('util/misc');
var ItemDef = require('data/items').ItemDef;
var ItemRow = require('ui/inventory').ItemRow;
var ToastList = require('ui/toast').ToastList;


/** @constructor */
function InventoryUpdateList() {
    this.toast = new ToastList('inv-update-list', 10, 5000);
    this.container = this.toast.dom;
    this.inv = null;
    this.skip = false;
}
exports.InventoryUpdateList = InventoryUpdateList;

InventoryUpdateList.prototype._fire = function(item_id, delta) {
    var def = ItemDef.by_id[item_id];

    var delta_str;
    if (delta == 0) {
        return;
    } else if (delta < 0) {
        delta_str = '\u2212' + (-delta);
    } else {
        delta_str = '+' + delta;
    }

    var row = new ItemRow(item_id, delta_str, def.ui_name, def.tile_x, def.tile_y);
    this.toast.add(row);
};

InventoryUpdateList.prototype.attach = function(inv) {
    if (this.inv != null) {
        this.inv.release();
    }
    this.inv = inv;

    var this_ = this;
    inv.onUpdate(function(idx, old_item, new_item) {
        // TODO: not correct for TAG_SPECIAL
        if (old_item.item_id == new_item.item_id) {
            this._fire(new_item.item_id, new_item.count - old_item.count);
        } else {
            this._fire(old_item.item_id, -old_item.count);
            this._fire(new_item.item_id, -new_item.count);
        }
    });
};
