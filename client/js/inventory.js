var fromTemplate = require('util').fromTemplate;

/** @constructor */
function InventoryUI() {
    this.list = new ItemList();
    this.container = fromTemplate('inventory', { 'item_list': this.list.container });

    this.list.container.classList.add('active');
}
exports.InventoryUI = InventoryUI;


/** @constructor */
function ItemList() {
    this.container = document.createElement('div');
    this.container.classList.add('item-list');

    this.rows = [];
    this.current_row = -1;
}
exports.ItemList = ItemList;

ItemList.prototype._setCurrentRow = function(new_idx) {
    var old_idx = this.current_row;

    if (old_idx != -1) {
        this.rows[old_idx].container.classList.remove('active');
    }

    if (new_idx != -1) {
        this.rows[new_idx].container.classList.add('active');
    }

    this.current_row = new_idx;
};

ItemList.prototype._scrollToFocus = function() {
    if (this.rows.length == 0) {
        return;
    }

    if (this.container.scrollHeight <= this.container.clientHeight) {
        return;
    }

    var idx = this.current_row;
    if (idx < 0) {
        return;
    }

    var item_height = this.rows[0].container.clientHeight;
    var viewport_height = this.container.clientHeight;
    this.container.scrollTop = (idx + 0.5) * item_height - 0.5 * viewport_height;
};

ItemList.prototype.addRows = function(rows) {
    rows.sort(function(a, b) { return a.id - b.id; });

    if (this.current_row != -1) {
        // Update the row index to keep the same ID selected.
        var current_id = this.rows[this.current_row].id;
        // The result of the binary search (findRow) will be the number of
        // items in 'rows' whose 'id' is less than 'current_id'.  Each such
        // item will be inserted before the current row by 'mergeInto'.
        this.current_row += findRow(rows, current_id);
    }

    var this_ = this;
    mergeInto(this.rows, rows, function(inserted, before) {
        if (before == null) {
            this_.container.appendChild(inserted.container);
        } else {
            this_.container.insertBefore(inserted.container, before.container);
        }
    });

    if (this.current_row != -1) {
        console.assert(current_id == this.rows[this.current_row].id,
                "id changed after insert");
    } else if (this.rows.length > 0) {
        this._setCurrentRow(0);
    }
    this._scrollToFocus();
};

ItemList.prototype.removeRows = function(ids) {
    ids.sort();

    var current_id;
    if (this.current_row != -1) {
        // Update the row index to keep the same ID selected.
        current_id = this.rows[this.current_row].id;
        this._setCurrentRow(-1);
    } else {
        current_id = -1;
    }


    var this_ = this;
    removeFrom(this.rows, ids, function(removed) {
        this_.container.removeChild(removed.container);
    });

    if (current_id != -1) {
        var new_row = findRow(this.rows, current_id);
        if (new_row >= this.rows.length) {
            new_row = this.rows.length - 1;
        }
        this._setCurrentRow(new_row);
    }
    this._scrollToFocus();
};

ItemList.prototype.step = function(offset) {
    var start_idx = this.current_row;
    if (start_idx < 0) {
        start_idx = 0;
    }

    var new_idx = start_idx + offset;
    if (new_idx < 0) {
        new_idx = 0;
    } else if (new_idx >= this.rows.length) {
        new_idx = this.rows.length - 1;
    }

    this._setCurrentRow(new_idx);
    this._scrollToFocus();
};


/** @constructor */
function ItemRow(id, qty, name, icon_x, icon_y) {
    this.container = document.createElement('div');
    this.container.classList.add('item');

    var quantityDiv = document.createElement('div');
    quantityDiv.classList.add('item-qty');
    quantityDiv.textContent = '' + qty;
    this.container.appendChild(quantityDiv);
    this.quantityDiv = quantityDiv;

    var iconDiv = document.createElement('div');
    iconDiv.classList.add('item-icon');
    iconDiv.style.backgroundPosition = '-' + icon_x + 'rem -' + icon_y + 'rem';
    this.container.appendChild(iconDiv);

    var nameDiv = document.createElement('div');
    nameDiv.classList.add('item-name');
    nameDiv.textContent = name;
    this.container.appendChild(nameDiv);

    this.id = id;
    this.qty = qty;
}
exports.ItemRow = ItemRow;

ItemRow.prototype.setQuantity = function(qty) {
    this.qty = qty;
    this.quantity.textContent = '' + qty;
};


// Given two lists 'a' and 'b' sorted by their 'id' fields, merge the items of
// 'b' into 'a', preserving the sorting.
function mergeInto(a, b, callback) {
    // Walk backwards over the output array, filling in the new values, which
    // are drawn from either 'b' or the lower-indexed parts of 'a'.
    var i = a.length - 1;
    var j = b.length - 1;
    var k = a.length + b.length - 1;
    a.length += b.length;

    while (i >= 0 && j >= 0) {
        // Use <= so that in case of identical IDs, we keep the ones from 'a'
        // in their original positions, and put the ones from 'b' after it.
        if (a[i].id <= b[j].id) {
            a[k] = b[j];
            callback(a[k], a[k + 1] || null);
            --j;
            --k;
        } else {
            a[k] = a[i];
            --i;
            --k;
        }
    }

    while (j >= 0) {
        a[k] = b[j];
        callback(a[k], a[k + 1] || null);
        --j;
        --k;
    }

    // No need for a loop to get the final elements of 'a'.  If 'j < 0', then
    // 'i == k', and we'd just be copying 'a[0] = a[0]', 'a[1] = a[1]', etc.

    return a;
}

// Given a sorted list of rows 'a' and a sorted list of IDs 'b', remove each
// item from 'a' whose ID appears in 'b'.
function removeFrom(a, b, callback) {
    var i = 0;
    var j = 0;
    var k = 0;

    while (i < a.length && j < b.length) {
        if (a[i].id == b[j]) {
            callback(a[i]);
            ++i;
        } else if (a[i].id < b[j]) {
            a[k] = a[i];
            ++i;
            ++k;
        } else /* a[i].id > b[j] */ {
            ++j;
        }
    }

    while (i < a.length) {
        a[k] = a[i];
        ++i;
        ++k;
    }

    a.length = k;

    return a;
}

function test_mergeInto_removeFrom() {
    function run_merge(a_id, b_id) {
        var a = a_id.map(function(x) { return ({ id: x }); });
        var b = b_id.map(function(x) { return ({ id: x }); });
        return mergeInto(a, b, function() {}).map(function(x) { return x.id; });
    }

    function run_remove(a_id, b_id) {
        var a = a_id.map(function(x) { return ({ id: x }); });
        return removeFrom(a, b_id, function() {}).map(function(x) { return x.id; });
    }

    function check(a, b, c) {
        var l1 = run_merge(a, b).toString();
        var l2 = run_merge(b, a).toString();
        var r = c.toString();
        console.assert(l1 == r,
                'mergeInto test failure: ' + a + ' + ' + b + ' = ' + l1 + ', not ' + r);
        console.assert(l2 == r,
                'mergeInto test failure: ' + b + ' + ' + a + ' = ' + l2 + ', not ' + r);

        var l = run_remove(c, a).toString();
        var r = b.toString();
        console.assert(l == r,
                'removeFrom test failure: ' + c + ' - ' + a + ' = ' + l + ', not ' + r);

        var l = run_remove(c, b).toString();
        var r = a.toString();
        console.assert(l == r,
                'removeFrom test failure: ' + c + ' - ' + b + ' = ' + l + ', not ' + r);
    }

    // Basic functionality
    check([1, 3, 5], [2, 4], [1, 2, 3, 4, 5]);

    // Insertion at the beginning, among others
    check([1, 3, 5], [0, 2, 4], [0, 1, 2, 3, 4, 5]);

    // Insertion solely at the beginning/end
    check([2, 3], [0, 1], [0, 1, 2, 3]);

    // Insertion in a block in the middle
    check([0, 1, 4, 5], [2, 3], [0, 1, 2, 3, 4, 5]);

    // Empty lists
    check([0, 1, 2, 3], [], [0, 1, 2, 3]);
}


function findRow(a, id) {
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

function test_findRow() {
    function run(a_id, id) {
        var a = a_id.map(function(x) { return ({ id: x }); });
        return findRow(a, id);
    }

    function check(a, id, expect) {
        var l = run(a, id);
        var r = expect;
        console.assert(l == r,
                'findRow test failure: find([' + a + '], ' + id + ' = ' + l + ', not ' + r);
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
