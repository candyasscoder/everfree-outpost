var Deque = require('util').Deque;

/** @constructor */
function TimeSeries(dur) {
    this._q = new Deque();
    this._dur = dur;
    this.sum = 0;
    this.count = 0;
    this._last_popped_time = Date.now();
}
exports.TimeSeries = TimeSeries;

TimeSeries.prototype.record = function(now, value) {
    var start = now - this._dur;
    while (true) {
        var item = this._q.peek();
        if (item == null) {
            break;
        }
        if (item[0] >= start) {
            break;
        }

        this._q.dequeue();
        this.sum -= item[1];
        --this.count;
        this._last_popped_time = item[0];
    }

    this._q.enqueue([now, value]);
    this.sum += value;
    ++this.count;
};

TimeSeries.prototype.duration = function() {
    return this._q.peek_back()[0] - this._last_popped_time;
};
