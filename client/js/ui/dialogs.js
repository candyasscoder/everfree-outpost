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


/** @constructor */
function TeleportSetupDialog(parts) {
    var dom_parts = util.templateParts('teleport-setup', {});

    var this_ = this;

    var submit = new widget.Button(dom_parts['submit']);
    submit.onclick = function() { this_.submit(); };
    var cancel = new widget.Button(dom_parts['cancel']);
    cancel.onclick = function() { this_.cancel(); };
    var buttons = new widget.SimpleList(dom_parts['buttons'], [submit, cancel],
            ['move_left', 'move_right']);

    this.name_input = new widget.TextField(dom_parts['name']);
    this.network_input = new widget.TextField(dom_parts['network']);

    var name_row = new widget.Container(dom_parts['name-row'], this.name_input);
    var network_row = new widget.Container(dom_parts['network-row'], this.network_input);

    var main = new widget.SimpleList(dom_parts['top'],
            [name_row, network_row, buttons]);
    widget.Form.call(this, main);
}
TeleportSetupDialog.prototype = Object.create(widget.Form.prototype);
TeleportSetupDialog.prototype.constructor = TeleportSetupDialog;

TeleportSetupDialog.prototype.submit = function() {
    if (this.onsubmit != null) {
        this.onsubmit({
            'name': this.name_input.dom.value,
            'network': this.network_input.dom.value,
        });
    }
};


/** @constructor */
function TeleportDestDialog(parts) {
    var dests = parts['dests'];
    dests.sort();

    var this_ = this;
    var items = [];
    var list_elt = util.element('div', ['list']);
    for (var i = 0; i < dests.length; ++i) {
        var elt = util.element('div', ['list-item', 'text=' + dests[i]], list_elt);
        items[i] = new widget.Button(elt);
        items[i].value = dests[i];
        items[i].onclick = function() { this_.submit(); };
    }
    this.list = new widget.SimpleList(list_elt, items);

    var dom = util.fromTemplate('teleport-dest', {'list': this.list.dom});
    widget.Form.call(this, this.list, dom);
}
TeleportDestDialog.prototype = Object.create(widget.Form.prototype);
TeleportDestDialog.prototype.constructor = TeleportDestDialog;

TeleportDestDialog.prototype.submit = function() {
    if (this.onsubmit != null) {
        this.onsubmit({
            'dest': this.list.selection().value,
        });
    }
};


exports.DIALOG_TYPES = [
    SignTextDialog,
    TeleportSetupDialog,
    TeleportDestDialog,
];
