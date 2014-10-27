var Vec = require('vec').Vec;
var Asm = require('asmlibs').Asm;
var TileDef = require('chunk').TileDef;
var CHUNK_SIZE = require('chunk').CHUNK_SIZE;
var LOCAL_SIZE = require('chunk').LOCAL_SIZE;

var INT_MAX = 0x7fffffff;
var INT_MIN = -INT_MAX - 1;


/** @constructor */
function Physics() {
    var chunk_total = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;
    var local_total = LOCAL_SIZE * LOCAL_SIZE;
    this._asm = new Asm(chunk_total * local_total);
}
exports.Physics = Physics;

Physics.prototype.loadChunk = function(ci, cj, tiles) {
    var view = this._asm.chunkShapeView(ci * LOCAL_SIZE + cj);
    console.assert(tiles.length == view.length,
            'expected ' + view.length + ' tiles, but got ' + tiles.length);

    for (var i = 0; i < tiles.length; ++i) {
        view[i] = TileDef.by_id[tiles[i]].shape;
    }
};

Physics.prototype.resetForecast = function(now, f, v) {
    this._step(now, f);
    f.target_v = v;
    this._forecast(f);
};

Physics.prototype.updateForecast = function(now, f) {
    var i;
    var LIMIT = 5;
    for (i = 0; i < LIMIT && !f.live(now); ++i) {
        var old_end_time = f.end_time;

        var time = Math.min(now, f.end_time);
        this._step(time, f);
        this._forecast(f);

        if (f.end_time == old_end_time) {
            // No progress has been made.
            return;
        }
    }
};

// Step the forecast forward to the given time, and set actual velocity to zero.
Physics.prototype._step = function(time, f) {
    var pos = f.position(time);
    f.start = pos;
    f.end = pos;
    f.actual_v = new Vec(0, 0, 0);
    f.start_time = time;
    f.end_time = INT_MAX * 1000;
};

// Project the time of the next collision starting from start_time, and set
// velocities, end_time, and end position appropriately.
Physics.prototype._forecast = function(f) {
    var result = this._asm.collide(f.start, f.size, f.target_v);
    if (result.t == 0) {
        return;
    }
    f.end = new Vec(result.x, result.y, result.z);
    f.actual_v = f.end.sub(f.start).mulScalar(1000).divScalar(result.t);
    f.end_time = f.start_time + result.t;
};


/** @constructor */
function Forecast(pos, size) {
    this.start = pos;
    this.end = pos;
    this.size = size;
    this.target_v = new Vec(0, 0, 0);
    this.actual_v = new Vec(0, 0, 0);
    // Timestamps are unix time in milliseconds.  This works because javascript
    // numbers have 53 bits of precision.
    this.start_time = INT_MIN * 1000;
    this.end_time = INT_MAX * 1000;
}
exports.Forecast = Forecast;

Forecast.prototype.position = function(now) {
    if (now < this.start_time) {
        return this.start.clone();
    } else if (now >= this.end_time) {
        return this.end.clone();
    } else {
        var delta = now - this.start_time;
        var offset = this.actual_v.mulScalar(delta).divScalar(1000);
        return this.start.add(offset);
    }
};

Forecast.prototype.velocity = function() {
    return this.actual_v;
};

Forecast.prototype.target_velocity = function() {
    return this.target_v;
};

Forecast.prototype.live = function(now) {
    return now >= this.start_time && now < this.end_time;
};


window.physBenchmark = function() {
    return phys._asm.collide(new Vec(0, 0, 0), new Vec(32, 32, 32), new Vec(30, 0, 0));
};
