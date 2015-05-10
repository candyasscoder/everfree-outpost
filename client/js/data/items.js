/** @constructor */
function ItemDef_(id, info) {
    this.id = id;
    this.ui_name = info['ui_name'] || info['name'];
    this.tile_x = id & 0x1f;
    this.tile_y = id >> 5;
}

// Closure compiler doesn't like having static items on functions.
var ItemDef = {};
exports.ItemDef = ItemDef;

ItemDef.by_id = [];
ItemDef.by_name = {};

ItemDef.register = function(id, info) {
    if (info == null) {
        return;
    }

    var item = new ItemDef_(id, info);
    while (ItemDef.by_id.length <= item.id) {
        ItemDef.by_id.push(null);
    }
    ItemDef.by_id[item.id] = item;
    ItemDef.by_name[item.name] = item;
};
