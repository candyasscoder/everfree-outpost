var Config = require('config').Config;
var ItemDef = require('data/items').ItemDef;
var RecipeDef = require('data/recipes').RecipeDef;
var ItemList = require('ui/inventory').ItemList;
var fromTemplate = require('util/misc').fromTemplate;
var InventoryTracker = require('inventory').InventoryTracker;
var widget = require('ui/widget');
var util = require('util/misc');


/** @constructor */
function SignTextDialog(parts) {
    var dom_parts = util.templateParts('sign-text', {});

    var this_ = this;

    var submit = new widget.Button(dom_parts['submit']);
    submit.onclick = function() { this_.submit(); };
    var cancel = new widget.Button(dom_parts['cancel']);
    cancel.onclick = function() { this_.cancel(); };
    var buttons = new widget.SimpleList(dom_parts['buttons'], [submit, cancel],
            ['move_left', 'move_right']);

    this.input = new widget.TextField(dom_parts['input']);

    var main = new widget.SimpleList(dom_parts['top'], [this.input, buttons]);
    widget.Form.call(this, main);
}
SignTextDialog.prototype = Object.create(widget.Form.prototype);
SignTextDialog.prototype.constructor = SignTextDialog;

SignTextDialog.prototype.submit = function() {
    if (this.onsubmit != null) {
        this.onsubmit({ 'msg': this.input.dom.value });
    }
};


exports.DIALOG_TYPES = [
    SignTextDialog,
];
