var Animation = require('graphics/sheet').Animation;
var Sprite = require('graphics/renderer').Sprite;
var Deque = require('util/misc').Deque;


/** @constructor */
function Motion(pos) {
    this.start_pos = pos;
    this.end_pos = pos;
    this.start_time = 0;
    this.end_time = 1;
    this.anim_id = 0;
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
function Entity(sprite_base, anim_info, pos) {
    this._sprite_base = sprite_base;
    this._anim = new Animation();
    this._anim_info = anim_info;

    this._cur_motion = new Motion(pos);
    this._motions = new Deque();

    this._updateAnimation();
}
exports.Entity = Entity;

Entity.prototype._dequeueUntil = function(now) {
    var did_dequeue = false;
    while (this._motions.peek() != null && this._motions.peek().start_time <= now) {
        this._cur_motion = this._motions.dequeue();
        did_dequeue = true;
    }
    if (did_dequeue) {
        this._updateAnimation();
    }
};

Entity.prototype._updateAnimation = function() {
    var m = this._cur_motion;
    var info = this._anim_info[m.anim_id];
    console.assert(info != null, 'no anim_info with id', m.anim_id);

    this._anim.animate(info.i, info.j, info.len, info.fps, info.flip, m.start_time);
};

Entity.prototype.position = function(now) {
    this._dequeueUntil(now);
    return this._cur_motion.position(now);
};

Entity.prototype.getSprite = function(now) {
    this._dequeueUntil(now);
    var sprite = this._sprite_base.instantiate();
    this._anim.updateSprite(now, sprite);

    var pos = this._cur_motion.position(now);
    sprite.setPos(pos);

    return sprite;
};

Entity.prototype.queueMotion = function(motion) {
    this._motions.enqueue(motion);
};

Entity.prototype.translateMotion = function(offset) {
    this._cur_motion.translate(offset);
    this._motions.forEach(function(m) { m.translate(offset); });
};

Entity.prototype.resetMotion = function(m) {
    this._cur_motion = m;
    this._motions = new Deque();
    this._updateAnimation();
};

Entity.prototype.motionEndTime = function(now) {
    this._dequeueUntil(now);
    return this._cur_motion.end_time;
};

Entity.prototype.animId = function(now) {
    this._dequeueUntil(now);
    return this._cur_motion.anim_id;
};
