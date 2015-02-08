// UI widget for maintaining a sorted list of items.  Each item must contain an
// 'id' field (the key used for sorting) and a 'container' field (the DOM
// element used to display the item in the list).  Adding, removing, or
// modifying items can be done through the 'update' method.
/** @constructor */
function SortedList(cls) {
    this.container = document.createElement('div');
    if (cls != null) {
        this.container.classList.add(cls);
    }

    this.items = [];
}
exports.SortedList = SortedList;

// Apply updates to the list.  For each 'update' in the 'updates' array, this
// function invokes 'callback(update, old_item)', where 'item' is the existing
// item with id 'update.id', or null if no such item exists.  The callback
// should return an item with id 'update.id' (which may or may not be the same
// object as 'old_item'), or 'null' to remove the old item from the list.
SortedList.prototype.update = function(updates, callback) {
    updates.sort(function(a, b) { return a.id - b.id; });

    var old_items = this.items;
    var new_items = [];

    var i = 0;
    var j = 0;
    var last_dom = null;

    while (i < old_items.length && j < updates.length) {
        var old_id = old_items[i].id;
        var update_id = updates[j].id;

        if (old_id < update_id) {
            // Copying an old item that needs no update.
            new_items.push(old_items[i]);
            last_dom = old_items[i].container;
            ++i;
        } else {
            // Applying an update of some kind.

            var old_item;
            if (old_id > update_id) {
                // Inserting a new element.
                old_item = null;
            } else {
                // Changing or removing an existing element.
                old_item = old_items[i];
                ++i;
            }

            var new_item = callback(updates[j], old_item);
            if (new_item != null) {
                console.assert(new_item.id == update_id,
                        "callback produced a item with bad id");
            }
            ++j;

            if (old_item != null && new_item != null) {
                this.container.replaceChild(old_item.container, new_item.container);
                last_dom = new_item.container;
                new_items.push(new_item);
            } else if (old_item != null /* && new_item == null */) {
                this.container.removeChild(old_item.container);
            } else if (new_item != null /* && old_item == null */) {
                var next_dom = last_dom == null ?
                        this.container.firstChild : last_dom.nextSibling;
                this.container.insertBefore(new_item.container, next_dom);
                last_dom = new_item.container;
                new_items.push(new_item);
            }
            // Else old_item == null && new_item == null, in which case there's
            // nothing to do.
        }
    }

    while (i < old_items.length) {
        new_items.push(old_items[i]);
        ++i;
    }

    while (j < updates.length) {
        var new_item = callback(updates[j], null);
        ++j;

        if (new_item != null) {
            this.container.appendChild(new_item.container);
            new_items.push(new_item);
        }
    }

    this.items = new_items;
};

SortedList.prototype.length = function() {
    return this.items.length;
};

SortedList.prototype.get = function(index) {
    return this.items[index];
};

SortedList.prototype.indexOf = function(id) {
    return findId(this.items, id);
};

SortedList.prototype.indexOfExact = function(id) {
    var index = this.indexOf(id);
    if (index >= this.items.length || this.items[index].id != id) {
        return -1;
    } else {
        return index;
    }
};


function findId(a, id) {
    var low = 0;
    var high = a.length;

    while (low < high) {
        var mid = (low + high) >> 1;
        if (a[mid].id == id) {
            return mid;
        } else if (a[mid].id < id) {
            low = mid + 1;
        } else {
            high = mid;
        }
    }

    return low;
}

function test_findId() {
    function run(a_id, id) {
        var a = a_id.map(function(x) { return ({ id: x }); });
        return findId(a, id);
    }

    function check(a, id, expect) {
        var l = run(a, id);
        var r = expect;
        console.assert(l == r,
                'findId test failure: find([' + a + '], ' + id + ' = ' + l + ', not ' + r);
    }

    check([], 99, 0);
    check([1, 3, 5], 0, 0);
    check([1, 3, 5], 1, 0);
    check([1, 3, 5], 2, 1);
    check([1, 3, 5], 3, 1);
    check([1, 3, 5], 4, 2);
    check([1, 3, 5], 5, 2);
    check([1, 3, 5], 6, 3);
}


// A wrapper around SortedList that tracks a selected item.
/** @constructor */
function SelectionList(cls) {
    this.list = new SortedList(cls);
    this.container = this.list.container;

    this.target_id = 0;
    this.actual_index = -1;

    this.old_row = null;
    this.onchange = null;
}
exports.SelectionList = SelectionList;

SelectionList.prototype.update = function(updates, callback) {
    this._preUpdate();

    this.list.update(updates, callback);

    this._postUpdate();
};

SelectionList.prototype._preUpdate = function() {
    this.old_row = this.selection();
};

SelectionList.prototype._postUpdate = function() {
    var old_row = this.old_row;
    // Avoid keeping the row alive unnecessarily.
    this.old_row = null;

    var new_actual_index = this.indexOf(this.target_id);
    if (new_actual_index >= this.length()) {
        // If the list is empty, then 'length - 1' is equal to -1.
        new_actual_index = this.length() - 1;
    }
    this.actual_index = new_actual_index;

    var new_row = this.selection();
    if (old_row !== new_row) {
        if (old_row != null) {
            old_row.container.classList.remove('active');
        }
        if (new_row != null) {
            new_row.container.classList.add('active');
        }
        if (this.onchange != null) {
            this.onchange(new_row);
        }
    }
};

SelectionList.prototype.select = function(id) {
    this.target_id = id;
    this._preUpdate();
    this._postUpdate();
};

SelectionList.prototype.step = function(delta) {
    if (this.actual_index == -1) {
        return;
    }

    var new_index = this.actual_index + delta;
    if (new_index < 0) {
        new_index = 0;
    } else if (new_index >= this.length()) {
        new_index = this.length() - 1;
    }
    this.select(this.get(new_index).id);
};

SelectionList.prototype.length = function() {
    return this.list.length();
};

SelectionList.prototype.get = function(index) {
    return this.list.get(index);
};

SelectionList.prototype.indexOf = function(id) {
    return this.list.indexOf(id);
};

SelectionList.prototype.indexOfExact = function(id) {
    return this.list.indexOfExact(id);
};

SelectionList.prototype.selection = function() {
    if (this.actual_index == -1) {
        return null;
    } else {
        return this.list.get(this.actual_index);
    }
};

SelectionList.prototype.selectedIndex = function() {
    return this.actual_index;
};
