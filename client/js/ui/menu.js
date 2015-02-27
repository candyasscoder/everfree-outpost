var Config = require('config').Config;
var util = require('util/misc');
var widget = require('ui/widget');


/** @constructor */
function Menu(items) {
    var item_widgets = [];
    this._key_map = {};
    for (var i = 0; i < items.length; ++i) {
        var name = items[i][0];
        var onaction = items[i][1];

        var amp_index = name.indexOf('&');
        var html_name = name;
        var hotkey_code = null;
        if (amp_index != -1) {
            var hotkey = name[amp_index + 1];
            html_name = name.substr(0, amp_index) +
                '<u>' + hotkey + '</u>' + name.substr(amp_index + 2);

            hotkey_code = hotkey.toUpperCase().charCodeAt(0);
            console.assert(this._key_map[hotkey_code] == null,
                    'duplicate menu items with hotkey', hotkey);
        }

        var item = new MenuItem(html_name, onaction);
        if (hotkey_code != null) {
            this._key_map[hotkey_code] = item;
        }
        item_widgets.push(item);
    }

    this.list = new widget.SimpleList('menu-container', item_widgets);
    this.dom = this.list.dom;
    this.keys = this;

    this.dialog = null;
};
exports.Menu = Menu;

Menu.prototype.handleKey = function(down, evt) {
    var binding = Config.ui_keybindings.get()[evt.keyCode];
    switch (binding) {
        case 'select':
            if (down && !evt.repeat) {
                this.dialog.hide();
                this.list.selection().onaction();
            }
            return;
    }

    var item = this._key_map[evt.keyCode];
    if (down && !evt.repeat && item != null) {
        this.dialog.hide();
        item.onaction();
        return;
    }

    this.list.keys.handleKey(down, evt);
};

Menu.prototype.handleOpen = function(dialog) {
    this.dialog = dialog;
};


/** @constructor */
function MenuItem(html, onaction) {
    this.dom = util.element('div', ['menu-item']);
    this.dom.innerHTML = html;
    this.keys = widget.NULL_KEY_HANDLER;

    this.onaction = onaction;
    this.dom.onclick = onaction;
}
