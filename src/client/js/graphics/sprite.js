var Vec = require('util/vec').Vec;


/** @constructor */
function Sprite(appearance) {
    this.appearance = appearance;

    this.width = 0;
    this.height = 0;

    this.ref_x = 0;
    this.ref_y = 0;
    this.ref_z = 0;
    this.anchor_x = 0;
    this.anchor_y = 0;

    this.frame_sheet = 0;
    this.frame_i = 0;
    this.frame_j = 0;

    this.flip = false;
}
exports.Sprite = Sprite;

// Lots of fields to set, so use this goofy builder pattern sort of thing.
Sprite.prototype.setSize = function(w, h) {
    this.width = w;
    this.height = h;
    return this;
};

Sprite.prototype.setRefPosition = function(x, y, z) {
    this.ref_x = x;
    this.ref_y = y;
    this.ref_z = z;
    return this;
};

Sprite.prototype.setAnchor = function(x, y) {
    this.anchor_x = x;
    this.anchor_y = y;
    return this;
};

Sprite.prototype.setFrame = function(sheet, i, j) {
    this.frame_sheet = sheet;
    this.frame_i = i;
    this.frame_j = j;
    return this;
};

Sprite.prototype.setFlip = function(flip) {
    this.flip = flip;
    return this;
};


Sprite.prototype.refPosition = function() {
    return new Vec(this.ref_x, this.ref_y, this.ref_z);
};
