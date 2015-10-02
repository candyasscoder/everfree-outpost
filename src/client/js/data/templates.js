var TILE_SIZE = require('data/chunk').TILE_SIZE;
var Vec = require('util/vec').Vec;


/** @constructor */
function TemplateDef_(id, info, assets) {
    this.id = id;

    var size = info['size'];
    this.size = new Vec(size[0], size[1], size[2]);

    this.shape = info['shape'];
    this.layer = info['layer'];
    this.sheet = info['sheet'];
    this.display_size = info['display_size'];
    this.display_offset = info['offset'];
    this.model_offset = info['model_offset'];
    this.model_length = info['model_length'];
    this.flags = info['flags'] || 0;

    this.anim_size = info['anim_size'] || [0, 0];
    this.anim_offset = info['anim_offset'] || [0, 0];
    this.anim_pos = info['anim_pos'] || [0, 0];
    this.anim_length = info['anim_length'] || 0;
    this.anim_rate = info['anim_rate'] || 0;
    this.anim_oneshot = info['anim_oneshot'] || false;
    this.anim_sheet = info['anim_sheet'] || 0;

    this.light_pos = info['light_pos'] || [0, 0, 0];
    this.light_color = info['light_color'] || [0, 0, 0];
    this.light_radius = info['light_radius'] || 0;
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
