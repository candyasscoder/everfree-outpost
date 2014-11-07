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


exports.rle16Decode = function(input, output) {
    var j = 0;
    for (var i = 0; i < input.length; ++i) {
        var cur = input[i];
        if ((cur & 0xf000) == 0) {
            output[j] = cur;
            ++j;
        } else {
            var count = cur & 0x0fff;
            ++i;
            var value = input[i];

            for (var k = 0; k < count; ++k) {
                output[j] = value;
                ++j;
            }
        }
    }
    return j;
};


exports.decodeUtf8 = function(view) {
    var utf8_buffer = '';
    var saw_utf8 = false;
    for (var i = 0; i < len; ++i) {
        var byte_ = view[i];
        utf8_buffer += String.fromCharCode(byte_);
        if (byte_ >= 0x80) {
            saw_utf8 = true;
        }
    }

    if (saw_utf8) {
        utf8_buffer = decodeURIComponent(escape(utf8_buffer));
    }
    return utf8_buffer;
};
