var Config = require('config').Config;
var util = require('util/misc');
var widget = require('ui/widget');


/** @constructor */
function PonyEditor(name, draw) {
    var this_ = this;


    // Build DOM

    var parts = util.templateParts('pony-editor', {});

    var options = parts['options'];
    function addRow(label, choices, choice_labels) {
        var base = util.element('div', ['pony-row'], options);
        var label_text = label != '' ? label + ':' : '';
        util.element('div', ['pony-label', 'text=' + label_text], base);

        var items = new Array(choices.length);
        for (var i = 0; i < choices.length; ++i) {
            // Use `html=` instead of `text=` to allow for HTML entities.
            items[i] = util.element('div',
                    ['pony-option-cell', 'html=' + choices[i]], base);
        }

        if (choice_labels == null) {
            choice_labels = items.map(function(x, idx, arr) { return idx; });
        }
        return {base: base, items: items, values: choice_labels};
    }
    var sexParts =   addRow('',      ['&#x2642;', '&#x2640;']);
    var tribeParts = addRow('Tribe', ['E', 'P', 'U']);
    var maneParts =  addRow('Mane',  ['A', 'B', 'C']);
    var tailParts =  addRow('Tail',  ['A', 'B', 'C']);
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
            items[i].onkey = function(evt) {};
            items[i].value = parts.values[i];
        }
        var row = new widget.SimpleList(parts.base, items, ['move_left', 'move_right']);
        row.onchange = function() { this_._refresh(); };
        return row;
    }

    this.sex =   rowWidget(sexParts);
    this.tribe = rowWidget(tribeParts);
    this.mane =  rowWidget(maneParts);
    this.tail =  rowWidget(tailParts);
    this.red =   rowWidget(redParts);
    this.green = rowWidget(greenParts);
    this.blue =  rowWidget(blueParts);

    var done = new widget.Button(parts['done']);
    done.onclick = function() { this_.submit(); };

    var list = new widget.SimpleList(parts['top'],
            [nameRow, this.sex, this.tribe,
             this.mane, this.tail,
             this.red, this.green, this.blue,
             done]);

    widget.Form.call(this, list);


    // Canvas setup

    var canvas = parts['canvas'];
    var scale = document.body.dataset.scale * 2;
    canvas.width = 96 * scale;
    canvas.height = 96 * scale;

    this.ctx = canvas.getContext('2d');
    this.ctx.scale(scale, scale);
    this.ctx.mozImageSmoothingEnabled = false;
    this.ctx.webkitImageSmoothingEnabled = false;
    this.ctx.imageSmoothingEnabled = false;


    // Initial setup

    this.name.dom.value = name;
    var name_dom = this.name.dom;
    this.name.onfocus = util.chain(this.name.onfocus, function() {
        var len = name_dom.value.length;
        name_dom.setSelectionRange(len, len);
    });

    // Disable drawing during initialization.
    this.draw = function() { };

    this.name.value = name;

    function oldInit(choice, saved) {
        if (saved != null) {
            for (var i = 0; i < choice.length(); ++i) {
                if (choice.get(i).dom.textContent == saved) {
                    choice.setFocus(i);
                    break;
                }
            }
        } else {
            choice.setFocus((Math.random() * 3)|0);
        }
    }

    function init(choice, saved) {
        if (saved != null) {
            choice.setFocus(saved);
        } else {
            choice.setFocus((Math.random() * choice.length())|0);
        }
    }

    var old_settings = Config.last_appearance.get() || {};
    if ('mane' in old_settings) {
        init(this.sex, old_settings['sex']);
        init(this.tribe, old_settings['tribe']);
        init(this.mane, old_settings['mane']);
        init(this.tail, old_settings['tail']);
        init(this.red, old_settings['red']);
        init(this.green, old_settings['green']);
        init(this.blue, old_settings['blue']);
    } else {
        oldInit(this.tribe, old_settings['tribe']);
        oldInit(this.red, old_settings['red']);
        oldInit(this.green, old_settings['green']);
        oldInit(this.blue, old_settings['blue']);
        this.sex.setFocus(0);
        this.mane.setFocus(0);
        this.tail.setFocus(0);
    }

    // Now actually draw.
    this.draw = draw;
    this._refresh();
}
PonyEditor.prototype = Object.create(widget.Form.prototype);
PonyEditor.prototype.constructor = PonyEditor;
exports.PonyEditor = PonyEditor;

PonyEditor.prototype._getAppearanceInfo = function() {
    return {
        sex: this.sex.selection().value,
        tribe: this.tribe.selection().value,
        mane: this.mane.selection().value,
        tail: this.tail.selection().value,
        red: this.red.selection().value,
        green: this.green.selection().value,
        blue: this.blue.selection().value,

        eyes: 0,
    };
};

PonyEditor.prototype._refresh = function() {
    this.draw(this.ctx, this._getAppearanceInfo());
};

PonyEditor.prototype.submit = function() {
    console.log('submit');
    if (this.onsubmit != null) {
        var name = this.name.dom.value;
        this.onsubmit(name, this._getAppearanceInfo());
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
