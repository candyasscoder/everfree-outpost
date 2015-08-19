/** @constructor */
function TimeVarying(now, init, min, max, velocity) {
    this.min = min;
    this.max = max;
    this.velocity = velocity;

    this.last_time = now;
    this.last_val = init;
}
exports.TimeVarying = TimeVarying;

TimeVarying.prototype.get = function(now) {
    var delta = (now - this.last_time) / 1000;
    var val = this.last_val + this.velocity * delta;
    if (val > this.max) {
        return this.max;
    } else if (val < this.min) {
        return this.min;
    } else {
        return val;
    }
};

TimeVarying.prototype.setVelocity = function(now, velocity) {
    this.last_val = this.get(now);
    this.last_time = now;
    this.velocity = velocity;
};
