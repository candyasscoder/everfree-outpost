/** @constructor */
function AttachmentDef_(id, info) {
    this.id = id;
    this.name = info['name'];
    this.sprite_file = info['sprite_file'];
}

/** @constructor */
function AttachSlotDef_(id, info) {
    this.id = id;
    this.name = info['name'];

    this.variants_by_id = new Array(info['variants'].length);
    this.variants_by_name = {};
    for (var i = 0; i < info['variants'].length; ++i) {
        var attachment = new AttachmentDef_(i, info['variants'][i]);
        this.variants_by_id[attachment.id] = attachment;
        this.variants_by_name[attachment.name] = attachment;
    }
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
    AttachSlotDef.by_name[item.name] = item;
};
