var Sprite = require('graphics/sprite').Sprite;
var Deque = require('util/misc').Deque;
var AnimationDef = require('data/animations').AnimationDef;


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

Motion.prototype.getAnimation = function() {
    return new Animation(AnimationDef.by_id[this.anim_id], this.start_time);
};


/** @constructor */
function Animation(def, start_time) {
    this.def = def;
    this.start_time = start_time || 0;
}
exports.Animation = Animation;

Animation.prototype.frameInfo = function(now) {
    var delta = now - this.start_time;
    var frame = Math.floor(delta * this.def.fps / 1000) % this.def.length;

    return {
        i: this.def.offset_y,
        j: this.def.offset_x + frame,
        sheet: this.def.sheet_idx,
        flip: this.def.flip,
    };
};


/** @constructor */
function Entity(app, anim, pos) {
    this._appearance = app;

    this._cur_anim = anim;
    this._anims = new Deque();

    this._cur_motion = new Motion(pos);
    this._motions = new Deque();

    this._light_radius = 0;
    this._light_color = null;
}
exports.Entity = Entity;

Entity.prototype._dequeueUntil = function(now) {
    while (this._motions.peek() != null && this._motions.peek().start_time <= now) {
        this._cur_motion = this._motions.dequeue();
    }

    while (this._anims.peek() != null && this._anims.peek().start_time <= now) {
        this._cur_anim = this._anims.dequeue();
    }
};

Entity.prototype.position = function(now) {
    this._dequeueUntil(now);
    return this._cur_motion.position(now);
};

Entity.prototype.getSprite = function(now) {
    this._dequeueUntil(now);
    var pos = this._cur_motion.position(now);
    var frame = this._cur_anim.frameInfo(now);
    return this._appearance.buildSprite(pos, frame);
}

Entity.prototype.queueMotion = function(motion) {
    this._motions.enqueue(motion);
    this.queueAnimation(motion.getAnimation());
};

Entity.prototype.queueAnimation = function(anim) {
    this._anims.enqueue(anim);
};

Entity.prototype.translateMotion = function(offset) {
    this._cur_motion.translate(offset);
    this._motions.forEach(function(m) { m.translate(offset); });
};

Entity.prototype.reset = function(m, a) {
    this._cur_motion = m;
    this._motions = new Deque();

    this._cur_anim = m.getAnimation();
    this._anims = new Deque();
};

Entity.prototype.motionEndTime = function(now) {
    this._dequeueUntil(now);
    return this._cur_motion.end_time;
};

Entity.prototype.setLight = function(radius, color) {
    this._light_radius = radius;
    this._light_color = color;
};

Entity.prototype.getLight = function() {
    if (this._light_radius <= 0) {
        return null;
    } else {
        return {
            color: this._light_color,
            radius: this._light_radius,
        };
    }
};

Entity.prototype.setAppearance = function(app) {
    this._appearance = app;
};

Entity.prototype.animId = function(now) {
    this._dequeueUntil(now);
    return this._cur_motion.anim_id;
};
