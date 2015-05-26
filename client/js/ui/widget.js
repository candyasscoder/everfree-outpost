var Config = require('config').Config;
var util = require('util/misc');

/*
 * Most of the UI is built from "Widgets".  A Widget is an object with two
 * special fields:
 *  - this.dom: Contains a DOM element used to display the widget.
 *  - this.keys: Contains a KeyHandler.
 *
 * A KeyHandler is an object with a 'handleKey(down, evt)' method.
 *
 * Some KeyHandlers and Widgets are defined below.
 */

/** @constructor */
function NullKeyHandler() {
}
exports.NullKeyHandler = NullKeyHandler;

NullKeyHandler.prototype.handleKey = function(down, evt) { };

var NULL_KEY_HANDLER = new NullKeyHandler();
exports.NULL_KEY_HANDLER = NULL_KEY_HANDLER;


/** @constructor */
function FocusTracker(items, key_names) {
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
exports.FocusTracker = FocusTracker;

FocusTracker.prototype.setFocus = function(idx) {
    if (this.items.length == 0) {
        return;
    }

    if (idx < 0) {
        idx = 0;
    }
    if (idx >= this.items.length) {
        idx = this.items.length - 1;
    }

    this.items[this.active].dom.classList.remove('active');
    this.active = idx;
    this.items[this.active].dom.classList.add('active');

    if (this.onchange != null) {
        this.onchange(idx);
    }
};

FocusTracker.prototype.handleKey = function(down, evt) {
    var mag = evt.shiftKey ? 10 : 1;

    var binding = Config.ui_keybindings.get(evt.keyCode)[evt.keyCode];
    if (binding == this.key_names[0]) {
        if (down) {
            this.setFocus(this.active - mag);
        }
        return true;
    } else if (binding == this.key_names[1]) {
        if (down) {
            this.setFocus(this.active + mag);
        }
        return true;
    } else if (this.items.length > 0) {
        return this.items[this.active].keys.handleKey(down, evt);
    }
};

FocusTracker.prototype.selection = function() {
    return this.items[this.active];
};

FocusTracker.prototype.selectedIndex = function() {
    return this.active;
};


/** @constructor */
function ActionKeyHandler(key_name, onaction, next_keys) {
    this.key_name = key_name;
    this.onaction = onaction;
    this.next_keys = next_keys || NULL_KEY_HANDLER;
}
exports.ActionKeyHandler = ActionKeyHandler;

ActionKeyHandler.prototype.handleKey = function(down, evt) {
    if (Config.ui_keybindings.get()[evt.keyCode] == this.key_name) {
        if (down && !evt.repeat && this.onaction != null) {
            this.onaction(evt);
        }
        return;
    }

    this.next_keys.handleKey(down, evt);
}


/** @constructor */
function SimpleList(cls, items, key_names) {
    this.dom = util.element('div', ['class=' + cls]);
    this.keys = new FocusTracker(items, key_names);

    for (var i = 0; i < items.length; ++i) {
        this.dom.appendChild(items[i].dom);
    }

    this.onchange = null;
    var this_ = this;
    this.keys.onchange = function(idx) {
        this_._handleChange(idx);
    };
}
exports.SimpleList = SimpleList;

SimpleList.prototype._handleChange = function(idx) {
    this._scrollToSelection();
    if (this.onchange != null) {
        this.onchange(idx);
    }
};

SimpleList.prototype._scrollToSelection = function() {
    var sel = this.keys.selection();
    if (sel == null) {
        return;
    }

    var item_bounds = sel.dom.getBoundingClientRect();
    var parent_bounds = this.dom.getBoundingClientRect();
    var target_top = parent_bounds.top + parent_bounds.height / 2 - item_bounds.height / 2;
    // Adjust scrollTop to move 'item_bounds.top' to 'target_top'.
    var delta = target_top - item_bounds.top;
    // Use -= like in ItemList
    this.dom.scrollTop -= delta;
};

SimpleList.prototype.selection = function() {
    return this.keys.selection();
};


/** @constructor */
function Element(dom) {
    this.dom = dom;
    this.keys = NULL_KEY_HANDLER;
}
exports.Element = Element;

/** @constructor */
function Template() {
    this.dom = util.fromTemplate.apply(null, arguments);
    this.keys = NULL_KEY_HANDLER;
}
exports.Template = Template;
