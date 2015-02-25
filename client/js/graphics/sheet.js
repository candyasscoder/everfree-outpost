/** @constructor */
function Animation() {
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

Animation.prototype.updateSprite = function(now, sprite) {
    var anim = this._anim;

    var delta = now - anim.start;
    var raw_frame = ((delta * anim.fps + (delta < 0 ? -999 : 0)) / 1000)|0;
    var frame = raw_frame % anim.len;
    if (frame < 0) {
        frame = (frame + anim.len) % anim.len;
    }

    sprite.extra.updateIJ(sprite, anim.i, anim.j + frame);
    sprite.setFlip(anim.flip);

    return sprite;
};
