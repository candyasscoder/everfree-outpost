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

InventoryUpdateList.prototype.attach = function(inv) {
    this.inv = inv;

    // Skip the first update, which just gives the initial contents of the
    // inventory.
    this.skip = true;

    var this_ = this;
    inv.onUpdate(function(updates) {
        if (this_.skip) {
            this_.skip = false;
            return;
        }

        for (var i = 0; i < updates.length; ++i) {
            var update = updates[i];
            var def = ItemDef.by_id[update.id];
            var delta = update.new_count - update.old_count;

            var delta_str;
            if (delta == 0) {
                continue;
            } else if (delta < 0) {
                delta_str = '\u2212' + (-delta);
            } else {
                delta_str = '+' + delta;
            }

            var row = new ItemRow(update.id, delta_str, def.ui_name, def.tile_x, def.tile_y);
            this_.toast.add(row);
        }
    });
};
