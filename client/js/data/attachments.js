
/** @constructor */
function AttachSlotDef_(id, info) {
    this.id = id;
    this.sprite_files = info['sprite_files'];
}

// Closure compiler doesn't like having static items on functions.
var AttachSlotDef = {};
exports.AttachSlotDef = AttachSlotDef;

AttachSlotDef.by_id = [];
AttachSlotDef.by_name = {};

AttachSlotDef.register = function(id, info) {
    if (info == null) {
        return;
    }

    var item = new AttachSlotDef_(id, info);
    while (AttachSlotDef.by_id.length <= item.id) {
        AttachSlotDef.by_id.push(null);
    }
    AttachSlotDef.by_id[item.id] = item;
};
