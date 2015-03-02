var util = require('util/misc');


/** @constructor */
function ToastList(cls, max, timeout) {
    this.dom = util.element('div', [cls]);
    this.items = new util.Deque();
    this.max = max;
    this.timeout = timeout;
    this.timer = null;
}
exports.ToastList = ToastList;

ToastList.prototype.add = function(row) {
    console.log('insert one');
    if (this.items.length() >= this.max) {
        console.log('remove due to len cap');
        this._remove();
    }

    var item = {
        dom: row.dom,
        timestamp: Date.now(),
    };
    this.items.enqueue(item);
    this.dom.appendChild(item.dom);
    this._updateTimer();
};

ToastList.prototype._remove = function() {
    var item = this.items.dequeue();
    if (item != null) {
        this.dom.removeChild(item.dom);
    }
};

// Clean up timed-out items and schedule another cleanup timer if items remain.
ToastList.prototype._updateTimer = function() {
    if (this.timer != null) {
        // Already have a timer pending.
        return;
    }

    var now = Date.now();
    var oldest = this.items.peek();
    while (oldest != null && oldest.timestamp + this.timeout < now) {
        this._remove();
        oldest = this.items.peek();
    }

    if (oldest == null) {
        // Queue is now empty.  No need to set a timer.
        return;
    }

    var delay = oldest.timestamp - now;

    var this_ = this;
    this.timer = setTimeout(function() {
        this_.timer = null;
        this_._updateTimer();
    }, delay);
};
