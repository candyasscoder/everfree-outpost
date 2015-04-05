var SimpleExtra = require('graphics/draw/simple').SimpleExtra;
var Vec = require('util/vec').Vec;


/** @constructor */
function TemplateDef_(id, info, assets) {
    this.id = id;

    var display_size = info['display_size'];
    var size = info['size'];
    var offset = info['offset'];

    var extra = new SimpleExtra(assets['structures' + info['sheet']]);
    extra.offset_x = offset[0];
    extra.offset_y = offset[1];

    var anchor_y = display_size[1] - size[1] * TILE_SIZE;
    this.base = new SpriteBase(display_size[0], display_size[1], 0, anchor_y, extra);

    this.size = new Vec(size[0], size[1], size[2]);
    this.shape = info['shape'];
}

// Closure compiler doesn't like having static items on functions.
var TemplateDef = {};
exports.TemplateDef = TemplateDef;

TemplateDef.by_id = [];

TemplateDef.register = function(id, info, assets) {
    if (info == null) {
        return;
    }

    var template = new TemplateDef_(id, info, assets);
    while (TemplateDef.by_id.length <= template.id) {
        TemplateDef.by_id.push(null);
    }
    TemplateDef.by_id[template.id] = template;
};
