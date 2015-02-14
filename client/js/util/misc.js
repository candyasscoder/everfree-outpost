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

Deque.prototype.length = function() {
    return this._cur.length + this._new.length;
};

Deque.prototype.forEach = function(f, thisArg) {
    this._cur.forEach(f, thisArg);
    this._new.forEach(f, thisArg);
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
    for (var i = 0; i < view.length; ++i) {
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


exports.buildArray = function(size, fn) {
    var a = new Array(size);
    for (var i = 0; i < size; ++i) {
        a[i] = fn();
    }
    return a;
};


exports.fromTemplate = function(id, parts) {
    var template = document.getElementById(id);
    console.assert(template != null, "no template with id", id);

    var copy = template.cloneNode(/* deep */ true);
    // Avoid duplicate IDs in the document.
    copy.removeAttribute('id');

    // Fill in the holes.
    var holes = copy.getElementsByClassName('hole');
    // Iterate in reverse since 'holes' is updated as the holes are removed
    // with 'replaceChild'.
    for (var i = holes.length - 1; i >= 0; --i) {
        var hole = holes[i];
        var key = hole.dataset.key;
        var part = parts[key];
        console.assert(part != null, 'missing part for template hole', key);
        hole.parentNode.replaceChild(part, hole);
    }

    return copy;
};


exports.chain = function(old, f) {
    if (old == null) {
        return f;
    }

    if (f == null) {
        return old;
    }

    return (function() {
        old.apply(this, arguments);
        f.apply(this, arguments);
    });
};


exports.element = function(tag, extra, parent) {
    var e = document.createElement(tag);

    for (var i = 0; i < extra.length; ++i) {
        if (extra[i].startsWith('#')) {
            e.setAttribute('id', extra[i].substr(1));
        } else {
            e.classList.add(extra[i]);
        }
    }

    if (parent != null) {
        parent.appendChild(e);
    }

    return e;
};
