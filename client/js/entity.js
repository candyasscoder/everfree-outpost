var Animation = require('sheet').Animation;
var Sprite = require('graphics').Sprite;


/** @constructor */
function Motion(pos) {
    this.start_pos = pos;
    this.end_pos = pos;
    this.start_time = 0;
    this.end_time = 1;
}
exports.Motion = Motion;

Motion.prototype.position = function(now) {
    var dur = this.end_time - this.start_time;
    var delta = Math.max(0, Math.min(dur, now - this.start_time));
    var offset = this.end_pos.sub(this.start_pos);
    return this.start_pos.add(offset.mulScalar(delta).divScalar(dur));
}

Motion.fromForecast = function(forecast, offset) {
    var m = new Motion(forecast.start.add(offset));
    m.end_pos = forecast.end.add(offset);
    m.start_time = forecast.start_time;
    m.end_time = forecast.end_time;
    return m;
};

Motion.prototype.translate = function(offset) {
    this.start_pos = this.start_pos.add(offset);
    this.end_pos = this.end_pos.add(offset);
};


/** @constructor */
function Entity(sheet, anim_info, pos, anchor) {
    this._anim = new Animation(sheet);
    this._anim_info = anim_info;
    this._motion = new Motion(pos);
    this._anchor_x = anchor.x;
    this._anchor_y = anchor.y;

    this.setAnimation(0, 0);
}
exports.Entity = Entity;

Entity.prototype.position = function(now) {
    return this._motion.position(now);
};

Entity.prototype.getSprite = function(now) {
    var cls = this._anim.sheet.getSpriteClass();
    var extra = this._anim.sheet.getSpriteExtra();
    var sprite = new Sprite();
    this._anim.updateSprite(now, sprite);

    var pos = this._motion.position(now);
    sprite.setDestination(pos, this._anchor_x, this._anchor_y);

    return sprite;
};

Entity.prototype.setAnimation = function(now, anim_id) {
    var info = this._anim_info[anim_id];
    console.assert(info != null, 'no anim_info with id', anim_id);

    this._anim.animate(info.i, info.j, info.len, info.fps, info.flip, now);
};

Entity.prototype.setMotion = function(motion) {
    this._motion = motion;
};

Entity.prototype.translateMotion = function(offset) {
    this._motion.translate(offset);
};
