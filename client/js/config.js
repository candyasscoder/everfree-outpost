var DEFAULT_CONFIG = {
    'show_controls': true,
    'ignore_browser_warning': false,
    'chat_scrollback': 100,

    'login_name': null,
    'login_secret': null,

    'keybindings': {
        37: 'move_left',    // ArrowLeft
        39: 'move_right',   // ArrowRight
        38: 'move_up',      // ArrowUp
        40: 'move_down',    // ArrowDown
        16: 'run',          // Shift
        65: 'interact',     // A
        68: 'use_item',     // D
        69: 'inventory',    // E
        112: 'show_controls', // F1
        113: 'show_menu',   // F2
        114: 'debug_show_panel', // F3
        115: 'debug_test',  // F4
        27: 'cancel',       // Esc
        32: 'cancel',       // Space
        13: 'chat',         // Enter
        191: 'chat_command',
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

    'debug_show_panel': false,
    'debug_timing_delay': [0, 25],
    'debug_force_mobile_warning': false,
    'debug_force_browser_warning': false,
};


exports.Config = {
    show_controls: new ConfigItem('show_controls'),
    ignore_browser_warning: new ConfigItem('ignore_browser_warning'),
    chat_scrollback: new ConfigItem('chat_scrollback'),

    login_name: new ConfigItem('login_name'),
    login_secret: new ConfigItem('login_secret'),

    keybindings: new ConfigItem('keybindings'),
    chat_keybindings: new ConfigItem('chat_keybindings'),
    ui_keybindings: new ConfigItem('ui_keybindings'),

    show_key_display: new ConfigItem('show_key_display'),

    debug_show_panel: new ConfigItem('debug_show_panel'),
    debug_timing_delay: new ConfigItem('debug_timing_delay'),
    debug_force_mobile_warning: new ConfigItem('debug_force_mobile_warning'),
    debug_force_browser_warning: new ConfigItem('debug_force_browser_warning'),
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
    localStorage.setItem(this.key, JSON.stringify(value));
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
