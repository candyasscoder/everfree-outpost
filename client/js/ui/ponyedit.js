var Config = require('config').Config;
var util = require('util/misc');
var widget = require('ui/widget');


/** @constructor */
function PonyEditor(name, draw) {
    var this_ = this;


    // Build DOM

    var parts = util.templateParts('pony-editor', {});

    var options = parts['options'];
    function addRow(label, choices) {
        var base = util.element('div', ['pony-row'], options);
        util.element('div', ['pony-label', 'text=' + label + ':'], base);

        var items = new Array(choices.length);
        for (var i = 0; i < choices.length; ++i) {
            items[i] = util.element('div',
                    ['pony-option-cell', 'text=' + choices[i]], base);
        }

        return {base: base, items: items, values: choices};
    }
    var tribeParts = addRow('Tribe', ['E', 'P', 'U']);
    var redParts =   addRow('Red',   ['1', '2', '3']);
    var greenParts = addRow('Green', ['1', '2', '3']);
    var blueParts =  addRow('Blue',  ['1', '2', '3']);

    this.message = parts['message'];


    // Build widgets

    this.name = new widget.TextField(parts['name-field']);
    var nameRow = new widget.Container(parts['name-row'], this.name);

    function rowWidget(parts) {
        var items = new Array(parts.items.length);
        for (var i = 0; i < parts.items.length; ++i) {
            items[i] = new widget.Button(parts.items[i]);
            items[i].value = parts.values[i];
        }
        var row = new widget.SimpleList(parts.base, items, ['move_left', 'move_right']);
        row.onchange = function() { this_._refresh(); };
        return row;
    }

    this.tribe = rowWidget(tribeParts);
    this.red =   rowWidget(redParts);
    this.green = rowWidget(greenParts);
    this.blue =  rowWidget(blueParts);

    var done = new widget.Button(parts['done']);
    done.onclick = function() { this_.submit(); };

    var list = new widget.SimpleList(parts['top'],
            [nameRow, this.tribe, this.red, this.green, this.blue, done]);

    widget.Form.call(this, parts['top'], list);


    // Canvas setup

    var canvas = parts['canvas'];
    var scale = document.body.dataset.scale;
    canvas.width = 96 * scale;
    canvas.height = 96 * scale;

    this.ctx = canvas.getContext('2d');
    this.ctx.scale(scale, scale);
    this.ctx.mozImageSmoothingEnabled = false;
    this.ctx.webkitImageSmoothingEnabled = false;
    this.ctx.imageSmoothingEnabled = false;


    // Initial setup

    this.name.dom.value = name;

    // Disable drawing during initialization.
    this.draw = function() { };

    this.name.value = name;

    function init(choice, saved) {
        if (saved != null) {
            for (var i = 0; i < choice.length(); ++i) {
                if (choice.get(i).value == saved) {
                    choice.setFocus(i);
                    break;
                }
            }
        } else {
            choice.setFocus((Math.random() * 3)|0);
        }
    }

    var old_settings = Config.last_appearance.get() || {};
    init(this.tribe, old_settings['tribe']);
    init(this.red, old_settings['red']);
    init(this.green, old_settings['green']);
    init(this.blue, old_settings['blue']);

    // Now actually draw.
    this.draw = draw;
    this._refresh();
}
PonyEditor.prototype = Object.create(widget.Form.prototype);
PonyEditor.prototype.constructor = PonyEditor;
exports.PonyEditor = PonyEditor;

PonyEditor.prototype._refresh = function() {
    var tribe = this.tribe.selection().value;
    var red = this.red.selection().value;
    var green = this.green.selection().value;
    var blue = this.blue.selection().value;
    this.draw(this.ctx, tribe, red, green, blue);
};

PonyEditor.prototype.submit = function() {
    console.log('submit');
    if (this.onsubmit != null) {
        var name = this.name.dom.value;
        var tribe = this.tribe.selection().value;
        var red = this.red.selection().value;
        var green = this.green.selection().value;
        var blue = this.blue.selection().value;
        this.onsubmit(name, tribe, red, green, blue);
    }
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
        // An error regarding the name.
        this.body.setFocus(0);
    }
};
