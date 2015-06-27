var Config = require('config').Config;
var util = require('util/misc');
var widget = require('ui/widget');
var getKeyName = require('util/keynames').getKeyName;


var KEYBIND_NAMES = [
    ['Movement', null],
    ['Up', 'move_up'],
    ['Down', 'move_down'],
    ['Left', 'move_left'],
    ['Right', 'move_right'],
    ['Run', 'run'],

    ['Actions', null],
    ['Interact', 'interact'],
    ['Use Ability', 'use_ability'],
    ['Use Item', 'use_item'],
    ['Open Abilities', 'abilities'],
    ['Open Inventory', 'inventory'],

    ['Hotbar', null],
    ['Slot 1', 'hotbar_1'],
    ['Slot 2', 'hotbar_2'],
    ['Slot 3', 'hotbar_3'],
    ['Slot 4', 'hotbar_4'],
    ['Slot 5', 'hotbar_5'],
    ['Slot 6', 'hotbar_6'],
    ['Slot 7', 'hotbar_7'],
    ['Slot 8', 'hotbar_8'],
    ['Slot 9', 'hotbar_9'],

    ['Misc', null],
    ['Show Controls', 'show_controls'],
    ['Open Menu', 'show_menu'],
    ['Placement Cursor', 'toggle_cursor'],
    ['Open Chat', 'chat'],
    ['Start Chat Command', 'chat_command'],
];


function buildInvBindingMap(bindings) {
    var ks = Object.getOwnPropertyNames(bindings);
    var result = {};
    for (var i = 0; i < ks.length; ++i) {
        var code = ks[i];
        var binding = bindings[code];

        result[binding] = code;
    }
    return result;
}

/** @constructor */
function KeybindingEditor(keyboard) {
    var this_ = this;

    var binding_map = Config.keybindings.get();
    var inv_binding_map = buildInvBindingMap(binding_map);
    var item_by_name = {};

    var items = [];
    var list_dom = util.element('div', ['keybinding-list']);
    for (var i = 0; i < KEYBIND_NAMES.length; ++i) {
        var display_name = KEYBIND_NAMES[i][0];
        var binding_name = KEYBIND_NAMES[i][1];

        if (binding_name == null) {
            var header = util.element('div', ['keybinding-header', 'text=' + display_name])
            list_dom.appendChild(header);
            continue;
        }

        var code = inv_binding_map[binding_name];
        var item = new KeybindingItem(display_name, code);
        items.push(item);
        list_dom.appendChild(item.dom);

        item_by_name[binding_name] = item;

        item.onclick = (function(binding_name) {
            return function() {
                this_.item_by_name[binding_name].setPending();
                this_.keyboard.pushHandler(function(down, evt) {
                    if (down) {
                        this_.bindKey(binding_name, evt.keyCode);
                        this_.keyboard.popHandler();
                    }
                    return true;
                });
            };
        })(binding_name);
    }

    var list = new widget.SimpleList(list_dom, items);

    var dom = util.fromTemplate('keybinding-editor', {'list': list.dom});
    widget.Form.call(this, list, dom);

    this.inv_binding_map = inv_binding_map;
    this.item_by_name = item_by_name;

    this.keyboard = keyboard;
}
KeybindingEditor.prototype = Object.create(widget.Form.prototype);
KeybindingEditor.prototype.constructor = KeybindingEditor;
exports.KeybindingEditor = KeybindingEditor;

KeybindingEditor.prototype.bindKey = function(binding, code) {
    var binding_map = Config.keybindings.get();

    var old_binding = binding_map[code];
    var old_code = this.inv_binding_map[binding];

    var item = this.item_by_name[binding];
    item.setKeyCode(code);
    this.inv_binding_map[binding] = code;
    binding_map[code] = binding;

    if (old_binding != null && old_binding != binding) {
        // A different action was bound to this key.  `binding_map` was already
        // updated.
        this.inv_binding_map[old_binding] = null;
        var old_binding_item = this.item_by_name[old_binding];
        if (old_binding_item != null) {
            old_binding_item.setKeyCode(null);
        }
    }

    if (old_code != null && old_code != code) {
        // A different key was bound to this action.  `inv_binding_map` was
        // already updated.
        delete binding_map[old_code];
    }

    Config.keybindings.save();
};


/** @constructor */
function KeybindingItem(name, code) {
    var parts = util.templateParts('keybinding-item', {});
    parts['name'].textContent = name;

    this.key_dom = parts['key'];
    this.setKeyCode(code);
    this.onclick = null;

    var button = new widget.Button(parts['key']);
    var this_ = this;
    button.onclick = function() {
        if (this_.onclick != null) {
            this_.onclick();
        }
    };

    widget.Container.call(this, parts['top'], button);
}
KeybindingItem.prototype = Object.create(widget.Container.prototype);
KeybindingItem.prototype.constructor = KeybindingItem;

KeybindingItem.prototype.setKeyCode = function(code) {
    if (code != null) {
        // Use `innerHTML` instead of `textContent` because the name may contain
        // entities (such as `&larr;`).
        this.key_dom.innerHTML = getKeyName(code);
    } else {
        this.key_dom.textContent = '(none)';
    }
};

KeybindingItem.prototype.setPending = function() {
    this.key_dom.textContent = '___';
};
