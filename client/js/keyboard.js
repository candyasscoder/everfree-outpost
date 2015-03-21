var Config = require('config').Config;


/** @constructor */
function Keyboard() {
    // Include a no-op handler, so we can always assume the stack is nonempty.
    this._handler_stack = [function() { return false; }];
    this.monitor = null;

    var debounce = Config.debounce_time.get();
    if (debounce > 0) {
        this._debounce_timers = {};
    } else {
        this._debounce_timers = null;
    }

    var this_ = this;

    this._keydown_listener = function(evt) {
        if (this_.monitor != null) {
            this_.monitor(true, evt);
        }

        if (this_._debounce_timers != null) {
            var timers = this_._debounce_timers;
            if (timers[evt.keyCode] != null) {
                clearTimeout(timers[evt.keyCode]);
                delete timers[evt.keyCode];
                evt.preventDefault();
                evt.stopPropagation();
                return;
            }
        }

        if (this_._topHandler()(true, evt) || alwaysStop(evt)) {
            evt.preventDefault();
            evt.stopPropagation();
        }
    };

    this._keyup_listener = function(evt) {
        if (this_.monitor != null) {
            this_.monitor(false, evt);
        }

        if (this_._debounce_timers != null) {
            var timers = this_._debounce_timers;
            timers[evt.keyCode] = setTimeout(function() {
                delete timers[evt.keyCode];
                this_._topHandler()(false, evt);
            }, Config.debounce_time.get());
            evt.preventDefault();
            evt.stopPropagation();
            return;
        }

        if (this_._topHandler()(false, evt) || alwaysStop(evt)) {
            evt.preventDefault();
            evt.stopPropagation();
        }
    };
}
exports.Keyboard = Keyboard;

function alwaysStop(evt) {
    // Allow Ctrl + anything
    if (evt.ctrlKey) {
        return false;
    }
    // Allow F5-F12
    if (evt.keyCode >= 111 + 5 && evt.keyCode <= 111 + 12) {
        return false;
    }

    // Allow typing in text fields.
    var focus_tag = document.activeElement.tagName.toLowerCase();
    if (focus_tag == 'input' || focus_tag == 'textarea') {
        return false;
    }

    // Stop all other events.
    return true;
}

Keyboard.prototype.pushHandler = function(handler) {
    this._handler_stack.push(handler);
}

Keyboard.prototype.popHandler = function() {
    this._handler_stack.pop();
    console.assert(this._handler_stack.length > 0);
}

Keyboard.prototype._topHandler = function() {
    var idx = this._handler_stack.length - 1;
    return this._handler_stack[idx];
}

Keyboard.prototype.attach = function(elt) {
    elt.addEventListener('keydown', this._keydown_listener);
    elt.addEventListener('keyup', this._keyup_listener);
}

Keyboard.prototype.detach = function(elt) {
    elt.removeEventListener('keydown', this._keydown_listener);
    elt.removeEventListener('keyup', this._keyup_listener);
}
