var TILE_SIZE = require('data/chunk').TILE_SIZE;


/** @constructor */
function Structure(pos, template, render_index) {
    this.pos = pos;
    this.template = template;

    this.render_index = render_index;
}
exports.Structure = Structure;
