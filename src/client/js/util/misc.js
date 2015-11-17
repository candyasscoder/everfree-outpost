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

Deque.prototype.forEach = function(f) {
    for (var i = this._cur.length - 1; i >= 0; --i) {
        f(this._cur[i]);
    }

    for (var i = 0; i < this._new.length; ++i) {
        f(this._new[i]);
    }
};


exports.fstr1 = function(x) {
    var y = Math.round(x * 10) / 10;
    if (y % 1 == 0) {
        return y + '.0';
    } else {
        return y + '';
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


function fromTemplate(id, parts) {
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
        var key = hole.dataset['key'];
        var part = parts[key];
        console.assert(part != null, 'missing part for template hole', key);
        hole.parentNode.replaceChild(part, hole);
    }

    return copy;
};
exports.fromTemplate = fromTemplate;

exports.templateParts = function(id, parts) {
    var copy = fromTemplate(id, parts);

    var result = {
        'top': copy,
    };
    function walk(node) {
        var part = node.dataset['part'];
        if (part != null) {
            result[part] = node;
        }

        for (var c = node.firstElementChild; c != null; c = c.nextElementSibling) {
            walk(c);
        }
    }
    walk(copy);
    return result;
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
    extra = extra || [];

    var elt = document.createElement(tag);

    for (var i = 0; i < extra.length; ++i) {
        var e = extra[i];
        var eq_idx = e.indexOf('=');
        if (eq_idx != -1) {
            var key = e.substr(0, eq_idx);
            var val = e.substr(eq_idx + 1);
            if (key == 'text') {
                elt.textContent = val;
            } else if (key == 'html') {
                elt.innerHTML = val;
            } else {
                elt.setAttribute(key, val);
            }
        } else if (e[0] == '#') {
            elt.setAttribute('id', e.substr(1));
        } else {
            elt.classList.add(e);
        }
    }

    if (parent != null) {
        parent.appendChild(elt);
    }

    return elt;
};
