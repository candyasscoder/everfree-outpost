var TimeSeries = require('util/timeseries').TimeSeries;
var fstr1 = require('util/misc').fstr1;


window['timeit'] = function(f) {
    var start = Date.now();
    var i = 0;
    while (Date.now() < start + 3000) {
        f();
        f();
        f();
        f();
        f();
        i += 5;
    }
    var end = Date.now();
    console.log(i + ' iterations in ' + (end - start) + ' ms = ' +
            fstr1((end - start) * 1000 / i) + ' us/iter');
};


/** @constructor */
function DebugMonitor() {
    this.container = document.createElement('table');
    this.container.setAttribute('class', 'debug-monitor');

    this.pos = this._addRow('Pos');
    this.fps = this._addRow('FPS');
    this.load = this._addRow('Load');
    this.jobs = this._addRow('Jobs');
    this.timing = this._addRow('Timing');
    this.motions = this._addRow('Motions');
    //this.plan = this._addRow('Plan');
    this.gfxDebug = this._addRow('Gfx');

    this._frames = new TimeSeries(5000);
    this._frame_start = 0;
}
exports.DebugMonitor = DebugMonitor;

DebugMonitor.prototype._addRow = function(label) {
    var row = document.createElement('tr');
    this.container.appendChild(row);

    var left = document.createElement('td');
    row.appendChild(left);
    left.textContent = label;

    var right = document.createElement('td');
    row.appendChild(right);
    return right;
};

DebugMonitor.prototype.frameStart = function() {
    this._frame_start = Date.now();
};

DebugMonitor.prototype.frameEnd = function() {
    var now = Date.now();
    this._frames.record(now, now - this._frame_start);

    var frames = this._frames.count;
    var dur = this._frames.duration() / 1000;
    var fps = frames / dur;
    this.fps.textContent =
        fstr1(fps) + ' fps (' + frames + ' in ' + fstr1(dur) + 's)';

    var work = this._frames.sum;
    var frame_work = work / frames;
    var frame_target = 16.6;
    var load = frame_work / frame_target * 100;
    this.load.textContent =
        fstr1(load) + '% (' + fstr1(frame_work) + ' / ' + fstr1(frame_target) + ')';
};

DebugMonitor.prototype.updateJobs = function(runner) {
    var counts = runner.count();
    var total = counts[0] + counts[1];
    this.jobs.textContent = total + ' (' + counts[0] + ' + ' + counts[1] + ')';
};

DebugMonitor.prototype.updatePlan = function(plan) {
    //this.plan.innerHTML = plan.map(describe_render_step).join('<br>');
};

DebugMonitor.prototype.updatePos = function(pos) {
    this.pos.innerHTML = pos.x + ', ' + pos.y + ', ' + pos.z;
};

DebugMonitor.prototype.updateTiming = function(timing) {
    var now = timing.visibleNow();
    var ping = timing.ping;
    var base = timing.client_base;
    this.timing.innerHTML = now + ' (Ping: ' + ping + 'ms)';
};

DebugMonitor.prototype.updateMotions = function(e, timing) {
    var motions = [];
    motions.push(e._cur_motion);
    for (var i = e._motions._cur.length - 1; i >= 0; --i) {
        motions.push(e._motions._cur[i]);
    }
    for (var i = 0; i < e._motions._new.length; ++i) {
        motions.push(e._motions._new[i]);
    }
    this.motions.innerHTML = motions
        .map(function(m) { return (m.start_time % 100000) + ' .. ' + (m.end_time % 100000); })
        .join('<br>');
};

DebugMonitor.prototype.updateGraphics = function(r) {
    this.gfxDebug.innerHTML = r.getDebugHTML();
};
