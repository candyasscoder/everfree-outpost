var CYCLE_LENGTH = 24000;

/** @constructor */
function DayNight(assets) {
    var info = assets['day_night'];
    this.sunrise = info['sunrise'];
    this.sunset = info['sunset'];

    // Adjust to put day_start at 0.
    var day_start = info['day_start'];
    this.day_start = 0;
    this.day_end = info['day_end'] - day_start;
    this.night_start = info['night_start'] - day_start;
    this.night_end = info['night_end'] - day_start;

    this.active = true;
    this.base_time = 0;
    this.cycle_ms = 24000;
}
exports.DayNight = DayNight;

DayNight.prototype._phaseTime = function(time) {
    time = (time|0) % CYCLE_LENGTH;

    if (time < this.day_end) {
        return [0, time];
    } else if (time < this.night_start) {
        return [1, time - this.day_end];
    } else if (time < this.night_end) {
        return [2, time - this.night_start];
    } else {
        return [3, time - this.night_end];
    }
};

DayNight.prototype.getAmbientColor = function(now) {
    if (!this.active) {
        return [0, 0, 0];
    }

    var pt = this._phaseTime((now - this.base_time) * CYCLE_LENGTH / this.cycle_ms);
    var phase = pt[0];
    var time = pt[1];
    if (phase == 0) {
        return [255, 255, 255];
    } else if (phase == 1) {
        var sunset_len = this.night_start - this.day_end;
        return interpolate(this.sunset, sunset_len - time - 1, sunset_len);
    } else if (phase == 2) {
        var night_len = this.night_end - this.night_start;
        return interpolate([this.sunset[0], this.sunrise[0]], time, night_len);
    } else if (phase == 3) {
        var sunrise_len = CYCLE_LENGTH - this.night_end;
        return interpolate(this.sunrise, time, sunrise_len);
    }
};

function interpolate(stops, x, max) {
    // 0     1=idx0  2=idx1    3
    // |-------|-------|-------|
    //         ^   ^   ^
    //         x0  x   x1
    //
    // a: weight of stops[idx0]
    // b: weight of stops[idx1]
    // d: denominator for weights


    var idx0 = ((stops.length - 1) * x / max)|0;
    var idx1 = idx0 + 1;

    var x0 = (max * idx0 / (stops.length - 1))|0;
    var x1 = (max * idx1 / (stops.length - 1))|0;

    var a = x1 - x;
    var b = x - x0;
    var d = x1 - x0;

    return [
        ((stops[idx0][0] * a + stops[idx1][0] * b) / d)|0,
        ((stops[idx0][1] * a + stops[idx1][1] * b) / d)|0,
        ((stops[idx0][2] * a + stops[idx1][2] * b) / d)|0,
            ];
}
