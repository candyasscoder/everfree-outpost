/** @constructor */
function Keyboard() {
    // Include a no-op handler, so we can always assume the stack is nonempty.
    this._handler_stack = [function() { return false; }];
}
exports.Keyboard = Keyboard;

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
    var this_ = this;

    elt.addEventListener('keydown', function(evt) {
        if (this_._topHandler()(true, evt)) {
            evt.preventDefault();
            evt.stopPropagation();
        }
    });

    elt.addEventListener('keyup', function(evt) {
        if (this_._topHandler()(false, evt)) {
            evt.preventDefault();
            evt.stopPropagation();
        }
    });
}
