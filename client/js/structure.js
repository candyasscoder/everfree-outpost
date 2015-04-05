var TILE_SIZE = require('data/chunk').TILE_SIZE;


/** @constructor */
function Structure(pos, px_pos, template) {
    this.pos = pos;
    this.template = template;

    var sprite = template.base.instantiate();
    sprite.ref_x = px_pos.x;
    sprite.ref_y = px_pos.y;
    sprite.ref_z = px_pos.z;

    if (template.layer != 0) {
        sprite.ref_y += template.size.y * TILE_SIZE;
    }
    this.sprite = sprite;
}
exports.Structure = Structure;
