/** @constructor */
function Sheet(image, item_width, item_height) {
    this.image = image;
    this.item_width = item_width;
    this.item_height = item_height;
}
exports.Sheet = Sheet;

Sheet.prototype.drawInto = function(ctx, i, j, x, y) {
    ctx.drawImage(this.image,
            j * this.item_width,
            i * this.item_height,
            this.item_width,
            this.item_height,
            x,
            y,
            this.item_width,
            this.item_height);
};

Sheet.prototype.updateSprite = function(sprite, i, j) {
    sprite.setSource(
            this.image,
            j * this.item_width,
            i * this.item_height,
            this.item_width,
            this.item_height);
};


/** @constructor */
function LayeredSheet(images, item_width, item_height) {
    this.images = images;
    this.item_width = item_width;
    this.item_height = item_height;
}
exports.LayeredSheet = LayeredSheet;

LayeredSheet.prototype.drawInto = function(ctx, i, j, x, y) {
    for (var idx = 0; idx < this.images.length; ++idx) {
        ctx.drawImage(this.images[idx],
                j * this.item_width,
                i * this.item_height,
                this.item_width,
                this.item_height,
                x,
                y,
                this.item_width,
                this.item_height);
    }
};

LayeredSheet.prototype.updateSprite = Sheet.prototype.updateSprite;


/** @constructor */
function Animation(sheet) {
    this.sheet = sheet;
    this._anim = null;
}
exports.Animation = Animation;

Animation.prototype.animate = function(i, j, len, fps, flip, now) {
    if (this._anim != null && i == this._anim.i && j == this._anim.j &&
            len == this._anim.len && fps == this._anim.fps && flip == this._anim.flip) {
        // The new animation is identical to the current one.  Let the
        // current one keep running so that the user doesn't see a skip.
        return;
    }

    this._anim = {
        i: i,
        j: j,
        len: len,
        fps: fps,
        flip: flip,
        start: now,
    };
};

Animation.prototype.drawAt = function(ctx, now, x, y) {
    var anim = this._anim;
    if (anim.flip) {
        ctx.scale(-1, 1);
        x = -x - this.sheet.item_width;
    }
    var frame = Math.floor((now - anim.start) * anim.fps / 1000) % anim.len;
    this.sheet.drawInto(ctx, anim.i, anim.j + frame, x, y);
    if (anim.flip) {
        ctx.scale(-1, 1);
    }
};

Animation.prototype.updateSprite = function(now, sprite) {
    var anim = this._anim;

    var delta = now - anim.start;
    var raw_frame = ((delta * anim.fps + (delta < 0 ? -999 : 0)) / 1000)|0;
    var frame = raw_frame % anim.len;
    if (frame < 0) {
        frame = (frame + anim.len) % anim.len;
    }

    this.sheet.updateSprite(sprite, anim.i, anim.j + frame);
    sprite.setFlip(anim.flip);
};
