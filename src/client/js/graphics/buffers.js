var glutil = require('graphics/glutil');
var LOCAL_SIZE = require('data/chunk').LOCAL_SIZE;

// Allow the actual buffer size to exceed the needed size by up to this amount.
// This allows the data size to shrink somewhat without forcing a reallocation.
//
// This number is large to avoid reallocation when the number of loaded chunks
// changes.
var BUFFER_MARGIN = 1024 * 1024;

// Allocate the actual buffer with this much extra space.  This allows the data
// size to grow somewhat without forcing a reallocation.
var BUFFER_EXTRA = 4 * 1024


/** @constructor */
function BufferCache(gl, callback) {
    this.gl = gl;
    this.buffer = new glutil.Buffer(gl);
    this.callback = callback;

    this.part_map = new Array(LOCAL_SIZE * LOCAL_SIZE);
    for (var i = 0; i < this.part_map.length; ++i) {
        this.part_map[i] = null;
    }
    this.first_part = null;
    this.last_part = null;

    this.buf_dirty = false;
    this.buf_size = -1;
    this.data_size = -1;

    this.last_cx0 = -1;
    this.last_cy0 = -1;
    this.last_cx1 = -1;
    this.last_cy1 = -1;
}
exports.BufferCache = BufferCache;

function calc_index(cx, cy) {
    return (cy & (LOCAL_SIZE - 1)) * LOCAL_SIZE + (cx & (LOCAL_SIZE - 1));
}

BufferCache.prototype.invalidate = function(cx, cy) {
    var n = this.part_map[calc_index(cx, cy)];
    if (n == null) {
        return;
    }

    n.dirty = true;
    this.buf_dirty = true;

    // Optimization: move the dirty buffer to the end of the list.  If it
    // changed once, it may change again soon (for example, a player may be
    // digging or building something large in this chunk).  Changes are less
    // expensive for the buffer on the end of the list, because we can often
    // avoid copying the data for the earlier (unmodified) parts.
    this._moveToEnd(n);
};

// Make sure the list of parts contains all visible chunks.
BufferCache.prototype._prepareList = function(cx0, cy0, cx1, cy1) {
    if (this.last_cx0 == cx0 && this.last_cy0 == cy0 &&
            this.last_cx1 == cx1 && this.last_cy1 == cy1) {
        return;
    }

    var idxs = {};
    for (var cy = cy0; cy < cy1; ++cy) {
        for (var cx = cx0; cx < cx1; ++cx) {
            idxs[calc_index(cx, cy)] = true;
        }
    }

    for (var n = this.first_part; n != null;) {
        var next = n.next;
        if (!idxs[n.chunk_idx]) {
            // Discard
            this._removeNode(n);
        }
        n = next;
    }

    for (var cy = cy0; cy < cy1; ++cy) {
        for (var cx = cx0; cx < cx1; ++cx) {
            var idx = calc_index(cx, cy);
            if (this.part_map[idx] == null) {
                this._addNode(idx, cx % LOCAL_SIZE, cy % LOCAL_SIZE);
            }
        }
    }

    this.last_cx0 = cx0;
    this.last_cy0 = cy0;
    this.last_cx1 = cx1;
    this.last_cy1 = cy1;
};



BufferCache.prototype._updateData = function(n) {
    var arrs = [];
    var total_size = 0;
    
    this.callback(n.cx, n.cy, function(arr) {
        if (arrs.length > 0) {
            console.assert(arr.constructor === arrs[0].constructor,
                    'callback provided multiple arrays of different types');
        }
        arrs.push(arr);
        total_size += arr.length;
    });

    if (total_size == 0) {
        return new Uint8Array(0);
    }
    var result = new (arrs[0].constructor)(total_size);
    var offset = 0;
    for (var i = 0; i < arrs.length; ++i) {
        result.set(arrs[i], offset);
        offset += arrs[i].length;
    }

    return result;
};

BufferCache.prototype._updateBuffer = function() {
    if (!this.buf_dirty) {
        return;
    }

    var gl = this.gl;

    // For each part, invoke the callback to update its data if necessary.
    // Also add up the total data size.
    var total_size = 0;
    for (var n = this.first_part; n != null; n = n.next) {
        if (n.dirty) {
            n.data = this._updateData(n);
        }
        total_size += n.data.byteLength;
    }

    // Is this a freshly allocated (empty) buffer?
    var fresh_buf = false;
    // Is the buffer already bound?
    var bound = false;

    // Reallocate the buffer if necessary.
    if (total_size > this.buf_size || total_size < this.buf_size - BUFFER_MARGIN) {
        this.buffer.bind();
        bound = true;
        
        this.buf_size = total_size + BUFFER_EXTRA;
        gl.bufferData(gl.ARRAY_BUFFER, this.buf_size, gl.STATIC_DRAW);
        fresh_buf = true;
    }
    this.data_size = total_size;

    // Store data into the buffer.
    var offset = 0;
    var s = [];
    for (var n = this.first_part; n != null; n = n.next) {
        if (n.dirty || n.last_offset != offset || fresh_buf) {
            // `n.data` is not already present in `this.buffer` at `offset`,
            // for one reason or another.
            if (!bound) {
                this.buffer.bind();
                bound = true;
            }
            gl.bufferSubData(gl.ARRAY_BUFFER, offset, n.data);
            n.dirty = false;
            n.last_offset = offset;
            s.push('!!');
        }
        offset += n.data.byteLength;

        s.push(n.chunk_idx + ': ' + n.data.byteLength + ' @ ' + n.last_offset);
    }
    console.log(s.join(';; '))

    // Done!

    if (bound) {
        this.buffer.unbind();
    }
    this.buf_dirty = false;
};

BufferCache.prototype.prepare = function(cx0, cy0, cx1, cy1) {
    this._prepareList(cx0, cy0, cx1, cy1);
    this._updateBuffer();
};

BufferCache.prototype.getBuffer = function() {
    return this.buffer;
};

BufferCache.prototype.getSize = function() {
    return this.data_size;
};

BufferCache.prototype._addNode = function(idx, cx, cy) {
    var n = new BufferNode(idx, cx, cy);
    n.link(this.last_part, null);
    if (n.prev == null) {
        this.first_part = n;
    }
    this.last_part = n;
    this.part_map[idx] = n;
    this.buf_dirty = true;
};

BufferCache.prototype._removeNode = function(n) {
    this.part_map[n.chunk_idx] = null;
    if (n.prev == null) {
        this.first_part = n.next;
    }
    if (n.next == null) {
        this.last_part = n.prev;
    }
    n.link(null, null);
    this.buf_dirty = true;
};

BufferCache.prototype._moveToEnd = function(n) {
    if (n.next == null) {
        // It's already at the end.
        return;
    }

    if (n.prev == null) {
        this.first_part = n.next;
    }
    n.link(this.last_part, null);
    this.last_part = n;
    // Don't need to update `first_part` ever.  We know there is at least one
    // element other than `n` in the list,  If there was only a single element,
    // then `n` would already be at the end.
};

/** @constructor */
function BufferNode(chunk_idx, cx, cy) {
    this.chunk_idx = chunk_idx;
    this.cx = cx;
    this.cy = cy;
    this.data = null;
    this.last_offset = 0;
    this.dirty = true;

    this.prev = null;
    this.next = null;
}

BufferNode.prototype.link = function(prev, next) {
    if (this.prev != null) {
        this.prev.next = this.next;
    }
    if (this.next != null) {
        this.next.prev = this.prev;
    }

    this.prev = prev;
    this.next = next;

    if (this.prev != null) {
        this.prev.next = this;
    }
    if (this.next != null) {
        this.next.prev = this;
    }
};
