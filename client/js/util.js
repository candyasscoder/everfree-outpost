/** @constructor */
function Deque() {
    this._cur = [];
    this._new = [];
}
exports.Deque = Deque;

Deque.prototype.enqueue = function(x) {
    this._new.push(x);
};

Deque.prototype.dequeue = function() {
    this._prepareRead();
    return this._cur.pop();
};

Deque.prototype._prepareRead = function() {
    if (this._cur.length == 0) {
        while (this._new.length > 0) {
            this._cur.push(this._new.pop());
        }
    }
};

Deque.prototype.peek = function() {
    this._prepareRead();
    if (this._cur.length == 0) {
        return null;
    } else {
        return this._cur[this._cur.length - 1];
    }
};

Deque.prototype.peek_back = function() {
    if (this._new.length > 0) {
        return this._new[this._new.length - 1];
    } else if (this._cur.length > 0) {
        return this._cur[0];
    } else {
        return null;
    }
};
