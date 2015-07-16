/** @constructor */
function AnimationDef_(id, info) {
    this.id = id;
    this.name = info['name'];
    this.offset_x = info['offset'][0];
    this.offset_y = info['offset'][1];
    this.length = info['length'];
    this.fps = info['framerate'];
    this.flip = info['mirror'];
    this.sheet_idx = info['sheet'];
}

// Closure compiler doesn't like having static items on functions.
var AnimationDef = {};
exports.AnimationDef = AnimationDef;

AnimationDef.by_id = [];
AnimationDef.by_name = {};

AnimationDef.register = function(id, info) {
    if (info == null) {
        return;
    }

    var item = new AnimationDef_(id, info);
    while (AnimationDef.by_id.length <= item.id) {
        AnimationDef.by_id.push(null);
    }
    AnimationDef.by_id[item.id] = item;
    AnimationDef.by_name[item.name] = item;
};
