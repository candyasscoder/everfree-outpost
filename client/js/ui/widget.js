var Config = require('config').Config;
var util = require('util/misc');

/*
 * Most of the UI is built from "Widgets".  A Widget is an object with two
 * special fields:
 *  - dom: Contains a DOM element used to display the widget.
 *  - onkey(evt): Function to handle a keydown or keyup event.
 *
 * Some KeyHandlers and Widgets are defined below.
 */


/** @constructor */
function WidgetKeyEvent(down, raw) {
    this.raw = raw;
    this.down = down;
    this.shiftKey = raw.shiftKey;
    this.useDefault = false;
};
exports.WidgetKeyEvent = WidgetKeyEvent;

WidgetKeyEvent.prototype.keyName = function() {
    return Config.keybindings.get()[this.raw.keyCode];
};

WidgetKeyEvent.prototype.chatKeyName = function() {
    return Config.chat_keybindings.get()[this.raw.keyCode];
};

WidgetKeyEvent.prototype.uiKeyName = function() {
    return Config.ui_keybindings.get()[this.raw.keyCode];
};

WidgetKeyEvent.prototype.requestDefault = function() {
    this.useDefault = true;
};


function hookKey(widget, name, hook) {
    hookKeys(widget, function(evt) {
        if (evt.uiKeyName() == name) {
            hook(evt);
            return true;
        }
    });
}
exports.hookKey = hookKey;

function hookKeys(widget, hook) {
    var old = widget.onkey;
    widget.onkey = function(evt) {
        if (hook(evt)) {
            return true;
        } else {
            return old.call(widget, evt);
        }
    };
}
exports.hookKeys = hookKeys;


function requestFocus(widget) {
    var w = widget;
    var p = w.parent;
    while (p != null) {
        if (p.onmessage != null) {
            return p.onmessage('request_focus', w);
        }
        w = p;
        p = w.parent;
    }
}


/** @constructor */
function Button(dom, trigger_key) {
    this.parent = null;
    this.dom = dom;
    this.onclick = null;
    this.trigger_key = trigger_key || 'select';

    var this_ = this;
    dom.onclick = function() {
        requestFocus(this_);
        this_.click();
    }
}
exports.Button = Button;

Button.prototype.click = function() {
    if (this.onclick != null) {
        this.onclick();
    }
};

Button.prototype.onkey = function(evt) {
    if (evt.uiKeyName() == this.trigger_key) {
        if (evt.down) {
            this.click();
        }
        return true;
    }
};


/** @constructor */
function TextField(dom) {
    this.parent = null;
    this.dom = dom;

    var this_ = this;
    dom.onclick = function() { requestFocus(this_); }
}
exports.TextField = TextField;

TextField.prototype.onkey = function(evt) {
    var code = evt.raw.keyCode;
    if (code == 0x20 || (code >= 0x40 + 1 && code <= 0x40 + 26)) {
        evt.requestDefault();
        return true;
    }
    return false;
};

TextField.prototype.onfocus = function() {
    this.dom.focus();
};

TextField.prototype.onblur = function() {
    this.dom.blur();
};


/** @constructor */
function Form(body, dom) {
    this.parent = null;
    this.dom = dom || body.dom;
    this.body = body;
    body.parent = this;

    this.onsubmit = null;
    this.oncancel = null;
}
exports.Form = Form;

Form.prototype.onkey = function(evt) {
    if (this.body.onkey(evt)) {
        return true;
    }

    var binding = evt.uiKeyName();
    if (binding == 'select') {
        if (evt.down) {
            this.submit();
        }
        return true;
    } else if (binding == 'cancel') {
        if (evt.down) {
            this.cancel();
        }
        return true;
    }
};

Form.prototype.submit = function() {
    if (this.onsubmit != null) {
        this.onsubmit();
    }
};

Form.prototype.cancel = function() {
    if (this.oncancel != null) {
        this.oncancel();
    }
};

Form.prototype.onfocus = function() {
    if (this.body.onfocus != null) {
        this.body.onfocus();
    }
};

Form.prototype.onblur = function() {
    if (this.body.onblur != null) {
        this.body.onblur();
    }
};


/** @constructor */
function Container(dom, body) {
    this.parent = null;
    this.dom = dom;
    this.body = body;
    this.body.parent = this;
}
exports.Container = Container;

Container.prototype.onkey = function(evt) {
    return this.body.onkey(evt);
};

Container.prototype.onfocus = function() {
    this.body.dom.classList.add('active');
    if (this.body.onfocus != null) {
        this.body.onfocus();
    }
};

Container.prototype.onblur = function() {
    this.body.dom.classList.remove('active');
    if (this.body.onblur != null) {
        this.body.onblur();
    }
};


/** @constructor */
function Element(dom) {
    this.parent = null;
    this.dom = dom;

    var this_ = this;
    dom.onclick = function() { requestFocus(this_); };
}
exports.Element = Element;

Element.prototype.onkey = function(evt) {};


/** @constructor */
function Template() {
    var dom = util.fromTemplate.apply(null, arguments);
    return new Element(dom);
}
exports.Template = Template;



/** @constructor */
function SimpleList(dom, items, key_names) {
    this.parent = null;
    this.dom = dom;

    for (var i = 0; i < items.length; ++i) {
        items[i].parent = this;
    }

    var this_ = this;
    dom.onclick = function() { requestFocus(this_); };

    this.items = items;
    this.key_names = key_names || ['move_up', 'move_down'];

    if (items.length > 0) {
        this.active = 0;
        this.items[this.active].dom.classList.add('active');
    } else {
        this.active = -1;
    }

    this.onchange = null;
}
exports.SimpleList = SimpleList;

SimpleList.prototype.onmessage = function(msg, origin) {
    if (msg == 'request_focus') {
        for (var i = 0; i < this.items.length; ++i) {
            if (this.items[i] === origin) {
                this.setFocus(i);
                break;
            }
        }
    }
};

SimpleList.prototype.onkey = function(evt) {
    if (this.active != -1 && this.selection().onkey(evt)) {
        return true;
    }

    var mag = evt.shiftKey ? 10 : 1;
    var binding = evt.uiKeyName();
    if (binding == this.key_names[0]) {
        if (evt.down) {
            this.setFocus(this.active - mag);
        }
        return true;
    } else if (binding == this.key_names[1]) {
        if (evt.down) {
            this.setFocus(this.active + mag);
        }
        return true;
    }
};

SimpleList.prototype._scrollToItem = function(sel) {
    var item_bounds = sel.dom.getBoundingClientRect();
    var parent_bounds = this.dom.getBoundingClientRect();

    var target_top = parent_bounds.top + parent_bounds.height / 2 - item_bounds.height / 2;
    // Adjust scrollTop to move 'item_bounds.top' to 'target_top'.
    var top_delta = target_top - item_bounds.top;
    // Use -= like in ItemList
    this.dom.scrollTop -= top_delta;

    var target_left = parent_bounds.left + parent_bounds.width / 2 - item_bounds.width / 2;
    var left_delta = target_left - item_bounds.left;
    this.dom.scrollLeft -= left_delta;
};

SimpleList.prototype.setFocus = function(idx) {
    if (this.items.length == 0) {
        this.active = -1;
        return;
    }

    if (idx < 0) {
        idx = 0;
    }
    if (idx >= this.items.length) {
        idx = this.items.length - 1;
    }


    var oldSel = this.selection();
    if (oldSel != null) {
        if (oldSel.onblur != null) {
            oldSel.onblur();
        }
        oldSel.dom.classList.remove('active');
    }

    this.active = idx;

    var newSel = this.selection();
    if (newSel != null) {
        if (newSel.onfocus != null) {
            newSel.onfocus();
        }
        newSel.dom.classList.add('active');
    }

    this._scrollToItem(newSel);


    if (this.onchange != null) {
        this.onchange(idx);
    }
};

SimpleList.prototype.length = function() {
    return this.items.length;
};

SimpleList.prototype.get = function(index) {
    return this.items[index];
};

SimpleList.prototype.selection = function() {
    return this.items[this.active];
};

SimpleList.prototype.selectedIndex = function() {
    return this.active;
};

SimpleList.prototype.onfocus = function() {
    if (this.active != -1) {
        var sel = this.selection();
        if (sel.onfocus != null) {
            sel.onfocus();
        }
    }
};

SimpleList.prototype.onblur = function() {
    if (this.active != -1) {
        var sel = this.selection();
        if (sel.onblur != null) {
            sel.onblur();
        }
    }
};


// A list widget to which items can be dynamically added and removed.
//
// NB: Unlike most widgets, which only attach behavior to existing DOM
// elements, this widget actually does manipulate the DOM itself.
/** @constructor */
function DynamicList(dom, key_names) {
    SimpleList.call(this, dom, [], key_names);
}
DynamicList.prototype = Object.create(SimpleList.prototype);
DynamicList.prototype.constructor = DynamicList;
exports.DynamicList = DynamicList;

// Apply updates to the list.  For each 'update' in the 'updates' array, this
// function invokes 'callback(update, old_item)', where 'item' is the existing
// item with id 'update.id', or null if no such item exists.  The callback
// should return an item with id 'update.id' (which may or may not be the same
// object as 'old_item'), or 'null' to remove the old item from the list.
DynamicList.prototype.update = function(updates, callback) {
    updates.sort(function(a, b) { return a.id - b.id; });

    var old_active_id = this.active != -1 ? this.items[this.active].id : -1;
    var old_items = this.items;
    var new_active_index = -1;
    var new_items = [];

    var this_ = this;
    function add(item) {
        item.parent = this_;
        new_items.push(item);
        if (item.id <= old_active_id) {
            new_active_index = new_items.length - 1;
        }
    }

    var i = 0;
    var j = 0;
    var last_dom = null;

    while (i < old_items.length && j < updates.length) {
        var old_id = old_items[i].id;
        var update_id = updates[j].id;

        if (old_id < update_id) {
            // Copying an old item that needs no update.
            add(old_items[i]);
            last_dom = old_items[i].dom;
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
                this.dom.replaceChild(new_item.dom, old_item.dom);
                last_dom = new_item.dom;
                add(new_item);
            } else if (old_item != null /* && new_item == null */) {
                this.dom.removeChild(old_item.dom);
            } else if (new_item != null /* && old_item == null */) {
                var next_dom = last_dom == null ?
                        this.dom.firstChild : last_dom.nextSibling;
                this.dom.insertBefore(new_item.dom, next_dom);
                last_dom = new_item.dom;
                add(new_item);
            }
            // Else old_item == null && new_item == null, in which case there's
            // nothing to do.
        }
    }

    while (i < old_items.length) {
        add(old_items[i]);
        ++i;
    }

    while (j < updates.length) {
        var new_item = callback(updates[j], null);
        ++j;

        if (new_item != null) {
            this.dom.appendChild(new_item.dom);
            add(new_item);
        }
    }

    this.items = new_items;
    this.active = new_active_index;
    // Call `setFocus` to apply the `active` class.  Note that we already
    // changed `this.active`, so the code doesn't actually know what the
    // previously selected element was.  This is okay because either the
    // previously selected element is the same as the newly selected element,
    // or the previously selected element has been removed from the list.
    this.setFocus(this.active);
};

DynamicList.prototype.indexOf = function(id) {
    return findId(this.items, id);
};

DynamicList.prototype.indexOfExact = function(id) {
    var index = this.indexOf(id);
    if (index >= this.items.length || this.items[index].id != id) {
        return -1;
    } else {
        return index;
    }
};

DynamicList.prototype.select = function(id) {
    var index = this.indexOf(id);
    if (index != -1) {
        this.setFocus(index);
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
