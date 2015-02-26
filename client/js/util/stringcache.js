// An LRU cache containing a set of strings.  Each string is mapped to a
// distinct index in the range [0..size).
/** @constructor */
function StringCache(size) {
    this.list = new CacheList(size);
    this.index = {};
}
exports.StringCache = StringCache;

StringCache.prototype.get = function(s) {
    var idx = this.index[s];
    if (idx != null) {
        this.list.hit(idx);
    }
    return idx;
};

StringCache.prototype.put = function(s) {
    var idx = this.list.last();
    this.index[s] = idx;
    this.list.hit(idx);
    return idx;
}


// A doubly-linked lists for implementing an LRU cache.
/** @constructor */
function CacheList(size) {
    this.list = new Uint16Array((size + 1) * 2);
    // Index of the sentinel nodes.
    this.marker = size;

    for (var i = 0; i < size; ++i) {
        this._setNext(i, i + 1);
    }
    // Note that item `size - 1` has its `next` pointer set to `size`, which
    // happens to equal `marker`.
    this._setNext(this.marker, 0);
}

CacheList.prototype._prev = function(idx) {
    return this.list[idx * 2 + 0];
};

CacheList.prototype._next = function(idx) {
    return this.list[idx * 2 + 1];
};

CacheList.prototype._setNext = function(idx, next) {
    this.list[idx * 2 + 1] = next;
    this.list[next * 2 + 0] = idx;
};

CacheList.prototype._relink = function(idx, new_prev, new_next) {
    var old_prev = this.list[idx * 2 + 0];
    var old_next = this.list[idx * 2 + 1];

    if (old_prev != 0xffff) {
        this.list[old_prev * 2 + 1] = old_next;
    }
    if (old_next != 0xffff) {
        this.list[old_next * 2 + 0] = old_prev;
    }

    this.list[idx * 2 + 0] = new_prev;
    this.list[idx * 2 + 1] = new_next;
};

CacheList.prototype._insertBefore = function(idx, before) {
    this._relink(idx, this._prev(before), before);
};

CacheList.prototype._insertAfter = function(idx, after) {
    this._relink(idx, after, this._next(after));
};

// Move an element to the end of the list.  (Mark the item as "most recently
// used".)
CacheList.prototype.hit = function(idx) {
    this._insertBefore(idx, this.marker);
};

// Move the first element of the list to the end, and return its index.  (Evict
// the least recently used item.)
CacheList.prototype.last = function() {
    return this._next(this.marker);
};
