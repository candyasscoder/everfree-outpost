var Config = require('config').Config;
var util = require('util/misc');
var widget = require('ui/widget');


/** @constructor */
function PonyEditor(name, draw) {
    var this_ = this;

    this.dom = util.fromTemplate('pony-editor', {});

    this.name = this.dom.getElementsByClassName('pony-name')[0];
    var nameWidget = {
        dom: this.dom.getElementsByClassName('pony-name-row')[0],
        keys: widget.NULL_KEY_HANDLER,
    };

    var table = this.dom.getElementsByClassName('pony-options')[0];
    this.tribe = new ChoiceRow('Tribe: ',    ['E', 'P', 'U']);
    this.red =   new ChoiceRow('Red: ',      ['1', '2', '3']);
    this.green = new ChoiceRow('Green: ',    ['1', '2', '3']);
    this.blue =  new ChoiceRow('Blue: ',     ['1', '2', '3']);
    var rows = [this.tribe, this.red, this.green, this.blue];
    for (var i = 0; i < rows.length; ++i) {
        var row = rows[i];
        table.appendChild(row.dom);
        row.onchange = function() { this_._refresh() };
    }

    var button = this.dom.getElementsByClassName('pony-done')[0];
    var buttonWidget = {
        dom: button,
        keys: new widget.ActionKeyHandler('select', function() {
            this_._finish();
        }),
    };


    this.keys = new widget.FocusTracker([
            nameWidget, this.tribe, this.red, this.green, this.blue, buttonWidget
    ]);

    this.keys.onchange = function(idx) {
        setTimeout(function() {
            if (idx == 0) {
                if (document.activeElement !== this_.name) {
                    this_.name.focus();
                    var len = this_.name.value.length;
                    this_.name.setSelectionRange(len, len);
                }
            } else {
                this_.name.blur();
            }
        }, 0);
    };

    this.name.onclick = function() { this_.keys.setFocus(0); };
    button.onclick = function() { this_._finish(); };


    this.message = this.dom.getElementsByClassName('pony-message')[0];


    var canvas = this.dom.getElementsByClassName('pony-display')[0];
    var scale = document.body.dataset.scale;
    canvas.width = 96 * scale;
    canvas.height = 96 * scale;

    this.ctx = canvas.getContext('2d');
    this.ctx.scale(scale, scale);
    this.ctx.mozImageSmoothingEnabled = false;
    this.ctx.webkitImageSmoothingEnabled = false;
    this.ctx.imageSmoothingEnabled = false;

    this.onfinish = null;
    this.dialog = null;


    // Disable drawing during initialization.
    this.draw = function() { };

    this.name.value = name;
    // TODO: remove this eventually
    if (name.substr(0, 4) == 'Anon' && name.length == 11) {
        this.tribe.setValue(name[4]);
        this.red.setValue(name[5]);
        this.green.setValue(name[6]);
        this.blue.setValue(name[7]);
    } else {
        var old_settings = Config.last_appearance.get() || {};
        function init(choice, saved) {
            if (saved != null) {
                choice.setValue(saved);
            } else {
                choice.setIndex((Math.random() * 3)|0);
            }
        }
        init(this.tribe, old_settings['tribe']);
        init(this.red, old_settings['red']);
        init(this.green, old_settings['green']);
        init(this.blue, old_settings['blue']);
    }

    // Now actually draw.
    this.draw = draw;
    this._refresh();
}
exports.PonyEditor = PonyEditor;

PonyEditor.prototype._refresh = function() {
    var tribe = this.tribe.getValue();
    var red = this.red.getValue();
    var green = this.green.getValue();
    var blue = this.blue.getValue();
    this.draw(this.ctx, tribe, red, green, blue);
};

PonyEditor.prototype._finish = function() {
    if (this.onfinish != null) {
        var name = this.name.value;
        var tribe = this.tribe.getValue();
        var red = this.red.getValue();
        var green = this.green.getValue();
        var blue = this.blue.getValue();
        this.onfinish(name, tribe, red, green, blue);
    }
};

PonyEditor.prototype.handleOpen = function(dialog) {
    this.dialog = dialog;
    this.keys.setFocus(0);

    // Capture and ignore the default 'close dialog' keybindings.
    var this_ = this;
    dialog.keyboard.pushHandler(function(down, evt) {
        this_.keys.handleKey(down, evt);
    });
};

PonyEditor.prototype.handleClose = function(dialog) {
    dialog.keyboard.popHandler();
};

PonyEditor.prototype._fixWidth = function() {
    // Explicitly set the message width, so that long messages don't stretch
    // the dialog.
    var width = this.dom.getBoundingClientRect().width;
    this.message.style.width = width + 'px';
};

PonyEditor.prototype.setMessage = function(msg) {
    this._fixWidth();
    this.message.classList.remove('error');
    this.message.textContent = msg;
};

PonyEditor.prototype.setError = function(code, msg) {
    this._fixWidth();
    this.message.classList.add('error');
    this.message.textContent = msg;
    if (code == 1) {
        this.keys.setFocus(0);
    }
};


/** @constructor */
function ChoiceRow(label, options) {
    this.dom = util.element('div', ['pony-row']);
    util.element('div', ['pony-label', 'text=' + label], this.dom);

    this.cells = [];
    for (var i = 0; i < options.length; ++i) {
        var cell = new ChoiceRowOption(options[i]);
        this.cells.push(cell);
        this.dom.appendChild(cell.dom);
    }

    this.keys = new widget.FocusTracker(this.cells, ['move_left', 'move_right']);

    this.onchange = null;
    var this_ = this;
    this.keys.onchange = function(idx) {
        if (this_.onchange != null) {
            this_.onchange(idx);
        }
    };
}

ChoiceRow.prototype.getValue = function() {
    return this.keys.selection().value;
};

ChoiceRow.prototype.setValue = function(value) {
    for (var i = 0; i < this.cells.length; ++i) {
        if (this.cells[i].value == value) {
            this.keys.setFocus(i);
            break;
        }
    }
};

ChoiceRow.prototype.setIndex = function(idx) {
    this.keys.setFocus(idx);
};

/** @constructor */
function ChoiceRowOption(label) {
    this.dom = util.element('div', ['pony-option-cell', 'text=' + label]);
    this.keys = widget.NULL_KEY_HANDLER;
    this.value = label;
}
