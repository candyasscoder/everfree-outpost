var Config = require('config').Config;
var util = require('util/misc');
var widget = require('ui/widget');


/** @constructor */
function Menu(items) {
    var this_ = this;

    var item_widgets = [];
    var dom = util.element('div', ['menu-container']);
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

        var item = (function(onaction) {
            return new MenuItem(html_name, function() {
                this_.cancel();
                onaction();
            });
        })(onaction);
        if (hotkey_code != null) {
            this._key_map[hotkey_code] = item;
        }
        item_widgets.push(item);
        dom.appendChild(item.dom);
    }

    var list = new widget.SimpleList(dom, item_widgets);
    widget.Form.call(this, dom, list);
};
Menu.prototype = Object.create(widget.Form.prototype);
Menu.prototype.constructor = Menu;
exports.Menu = Menu;

Menu.prototype.onkey = function(evt) {
    if (widget.Form.prototype.onkey.call(this, evt)) {
        return true;
    }

    var item = this._key_map[evt.raw.keyCode];
    if (item != null) {
        if (evt.down) {
            item.click();
        }
        return true;
    }
};


/** @constructor */
function MenuItem(html, onaction) {
    var dom = util.element('div', ['menu-item']);
    dom.innerHTML = html;
    widget.Button.call(this, dom);
    this.onclick = onaction;
}
MenuItem.prototype = Object.create(widget.Button.prototype);
MenuItem.prototype.constructor = MenuItem;
