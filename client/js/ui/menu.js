var SelectionList = require('ui/sortedlist').SelectionList;
var util = require('util/misc');


/** @constructor */
function Menu(items) {
    this.list = new SelectionList('menu-container');
    this.container = this.list.container;

    this.dialog = null;

    this._callbacks = [];
    this._key_map = {};

    var updates = [];
    for (var i = 0; i < items.length; ++i) {
        var name = items[i][0];
        this._callbacks.push(items[i][1]);

        var amp_index = name.indexOf('&');
        var html_name = name;
        if (amp_index != -1) {
            var hotkey = name[amp_index + 1];
            html_name = name.substr(0, amp_index) +
                '<u>' + hotkey + '</u>' + name.substr(amp_index + 2);

            var hotkey_code = hotkey.toUpperCase().charCodeAt(0);
            console.assert(this._key_map[hotkey_code] == null,
                    'duplicate menu items with hotkey', hotkey);
            this._key_map[hotkey_code] = i;
        }

        updates.push({
            id: i,
            html: html_name,
        });
    }

    var this_ = this;
    this.list.update(updates, function(up, old_row) {
        return new MenuItem(this_, up.id, up.html);
    });
    this.list.select(0);
};
exports.Menu = Menu;

Menu.prototype._handleKeyEvent = function(down, evt) {
    if (!down) {
        return;
    }


    var binding = Config.menu_keybindings.get()[evt.keyCode];
    switch (binding) {
        case 'select':
            this._handleChoice(this.list.selection().id);
            return;
        case 'cancel':
            this.dialog.hide();
            return;
        case 'move_up':
            this.list.step(-1);
            return;
        case 'move_down':
            this.list.step(1);
            return;
    }

    var choice = this._key_map[evt.keyCode];
    if (choice != null) {
        this._handleChoice(choice);
    }
};

Menu.prototype._handleChoice = function(idx) {
    this.dialog.hide();
    this._callbacks[idx]();
};

Menu.prototype.handleOpen = function(dialog) {
    var this_ = this;
    this.dialog = dialog;
    dialog.keyboard.pushHandler(function(d, e) { return this_._handleKeyEvent(d, e); });
};

Menu.prototype.handleClose = function(dialog) {
    this.dialog = null;
    dialog.keyboard.popHandler();
};


/** @constructor */
function MenuItem(owner, idx, html) {
    this.container = util.element('div', ['menu-item']);
    this.container.innerHTML = html;
    this.id = idx;

    this.container.onclick = function() { owner._handleChoice(idx); };
}
