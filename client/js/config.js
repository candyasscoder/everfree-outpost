var DEFAULT_CONFIG = {
    'show_controls': true,
    'show_inventory_updates': true,
    'ignore_browser_warning': false,
    'chat_scrollback': 100,
    'chat_lines': 8,
    'chat_autohide': false,
    'scale_world': 0,
    'scale_ui': 0,

    'render_outline': true,
    'render_names': true,

    'motion_prediction': true,
    'input_delay': 10,
    'debounce_time': 0,

    'login_name': null,
    'login_secret': null,
    'world_version': null,
    'last_appearance': null,

    'keybindings': {
        37: 'move_left',    // ArrowLeft
        39: 'move_right',   // ArrowRight
        38: 'move_up',      // ArrowUp
        40: 'move_down',    // ArrowDown
        16: 'run',          // Shift

        65: 'interact',     // A
        83: 'use_ability',  // S
        87: 'abilities',    // W
        68: 'use_item',     // D
        69: 'inventory',    // E

        112: 'show_controls', // F1
        113: 'show_menu',   // F2
        114: 'debug_show_panel', // F3
        115: 'debug_test',  // F4
        67: 'toggle_cursor', // C

        27: 'cancel',       // Esc
        32: 'cancel',       // Space
        13: 'chat',         // Enter
        191: 'chat_command', // '/'
    },

    'chat_keybindings': {
        13: 'send',         // Enter
        27: 'cancel',       // Esc
    },

    'ui_keybindings': {
        37: 'move_left',    // ArrowLeft
        39: 'move_right',   // ArrowRight
        38: 'move_up',      // ArrowUp
        40: 'move_down',    // ArrowDown
        27: 'cancel',       // Esc
        32: 'cancel',       // Space
        13: 'select',       // Enter
        65: 'select',       // A
    },

    'show_key_display': false,

    'ignores': {},

    'debug_show_panel': false,
    'debug_force_mobile_warning': false,
    'debug_force_browser_warning': false,
    'debug_block_webgl_extensions': {},
};


exports.Config = {
    show_controls: new ConfigItem('show_controls'),
    show_inventory_updates: new ConfigItem('show_inventory_updates'),
    ignore_browser_warning: new ConfigItem('ignore_browser_warning'),
    chat_scrollback: new ConfigItem('chat_scrollback'),
    chat_lines: new ConfigItem('chat_lines'),
    chat_autohide: new ConfigItem('chat_autohide'),
    scale_world: new ConfigItem('scale_world'),
    scale_ui: new ConfigItem('scale_ui'),

    render_outline: new ConfigItem('render_outline'),
    render_names: new ConfigItem('render_names'),

    motion_prediction: new ConfigItem('motion_prediction'),
    input_delay: new ConfigItem('input_delay'),
    debounce_time: new ConfigItem('debounce_time'),

    login_name: new ConfigItem('login_name'),
    login_secret: new ConfigItem('login_secret'),
    world_version: new ConfigItem('world_version'),
    last_appearance: new ConfigItem('last_appearance'),

    keybindings: new ConfigItem('keybindings'),
    chat_keybindings: new ConfigItem('chat_keybindings'),
    ui_keybindings: new ConfigItem('ui_keybindings'),

    show_key_display: new ConfigItem('show_key_display'),

    ignores: new ConfigItem('ignores'),

    debug_show_panel: new ConfigItem('debug_show_panel'),
    debug_force_mobile_warning: new ConfigItem('debug_force_mobile_warning'),
    debug_force_browser_warning: new ConfigItem('debug_force_browser_warning'),
    debug_block_webgl_extensions: new ConfigItem('debug_block_webgl_extensions'),
};


/** @constructor */
function ConfigItem(key) {
    this.key = key;
    this.value = null;
}

ConfigItem.prototype.get = function() {
    if (this.value == null) {
        var str = localStorage.getItem(this.key);
        if (!str) {
            this.value = DEFAULT_CONFIG[this.key];
        } else {
            this.value = JSON.parse(str);
        }
    }

    return this.value;
};

ConfigItem.prototype.set = function(value) {
    this.value = value;
    this.save();
};

ConfigItem.prototype.toggle = function(value) {
    var new_value = !this.get();
    this.set(new_value);
    return new_value;
};

ConfigItem.prototype.isSet = function() {
    return localStorage.getItem(this.key) != null;
};

ConfigItem.prototype.reset = function() {
    localStorage.removeItem(this.key);
    this.value = null;
};

ConfigItem.prototype.save = function() {
    localStorage.setItem(this.key, JSON.stringify(this.value));
};
