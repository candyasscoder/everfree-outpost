var Config = require('config').Config;
var ItemDef = require('data/items').ItemDef;
var RecipeDef = require('data/recipes').RecipeDef;
var SelectionList = require('ui/sortedlist').SelectionList;
var ItemList = require('ui/inventory').ItemList;
var fromTemplate = require('util/misc').fromTemplate;
var InventoryTracker = require('inventory').InventoryTracker;
var widget = require('ui/widget');
var element = require('util/misc').element;


/** @constructor */
function SignTextDialog(parts) {
    this.dom = element('div', []);
    element('div', ['title', 'text=Sign Text'], this.dom);

    var this_ = this;

    this.input = {
        dom: element('input', ['text-box', 'type=text', 'size=100'], this.dom),
        keys: {
            handleKey: function(down, evt) {
                if (evt.keyCode == 13) {
                    this_._finish();
                    return true;
                } else if (evt.keyCode == 27) {
                    this_.dialog.hide();
                    return true;
                } else {
                    return false;
                }
            }
        }
    };

    var button_row = element('div', [], this.dom);

    var finish_button = {
        dom: element('span', ['button', 'text=Done'], button_row),
        keys: new widget.ActionKeyHandler('select', function() { this_._finish(); }),
    };
    finish_button.onclick = function() { this_._finish(); };

    var cancel_button = {
        dom: element('span', ['button', 'text=Cancel'], button_row),
        keys: new widget.ActionKeyHandler('select', function() { this_.dialog.hide(); }),
    };
    cancel_button.onclick = function() { this_.dialog.hide(); };

    var row_widget = {
        dom: button_row,
        keys: new widget.FocusTracker([finish_button, cancel_button],
                      ['move_left', 'move_right']),
    };

    this.keys = new widget.FocusTracker([this.input, row_widget]);
    this.keys.onchange = function(idx) {
        if (idx == 0) {
            this_.input.dom.focus();
        } else {
            this_.input.dom.blur();
        }
    };

    this.dialog = null;
    this.onfinish = null;
}

SignTextDialog.prototype.handleOpen = function(dialog) {
    this.dialog = dialog;
    this.keys.setFocus(0);
    this.input.value = '';

    // Capture and ignore the default 'close dialog' keybindings.
    var this_ = this;
    dialog.keyboard.pushHandler(function(down, evt) {
        this_.keys.handleKey(down, evt);
    });
};

SignTextDialog.prototype.handleClose = function(dialog) {
    dialog.keyboard.popHandler();
};

SignTextDialog.prototype._finish = function() {
    if (this.onfinish != null) {
        this.onfinish({ 'msg': this.input.dom.value });
    }
    this.dialog.hide();
};


exports.DIALOG_TYPES = [
    SignTextDialog,
];
