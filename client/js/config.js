var DEFAULT_CONFIG = {
    'show_controls': true,

    'keybindings': {
        37: 'move_left',    // ArrowLeft
        39: 'move_right',   // ArrowRight
        38: 'move_up',      // ArrowUp
        40: 'move_down',    // ArrowDown
        16: 'run',          // Shift
        65: 'interact',     // A
        69: 'inventory',    // E
        112: 'show_controls', // F1
        27: 'cancel',       // Esc
        32: 'cancel',       // Space
    },
};


exports.Config = {
    show_controls: new BooleanConfigItem('show_controls'),
    keybindings: new JsonConfigItem('keybindings'),
};


/** @constructor */
function ConfigItem(key, from, to) {
    this.key = key;
    this.value = null;
    this.from_string = from;
    this.to_string = to;
}

ConfigItem.prototype.get = function() {
    if (this.value == null) {
        var str = localStorage.getItem(this.key);
        if (!str) {
            this.value = DEFAULT_CONFIG[this.key];
        } else {
            this.value = this.from_string(str);
        }
    }

    return this.value;
};

ConfigItem.prototype.set = function(value) {
    this.value = value;
    localStorage.setItem(this.key, this.to_string(value));
};

ConfigItem.prototype.toggle = function(value) {
    var new_value = !this.get();
    this.set(new_value);
    return new_value;
};


function StringConfigItem(key) {
    function from_string(s) {
        return s;
    }

    function to_string(v) {
        return v;
    }

    return new ConfigItem(key, from_string, to_string);
}

function NumberConfigItem(key) {
    function from_string(s) {
        return +s;
    }

    function to_string(v) {
        return v.toString();
    }

    return new ConfigItem(key, from_string, to_string);
}

function BooleanConfigItem(key) {
    function from_string(s) {
        return s != '0';
    }

    function to_string(v) {
        return v ? '1' : '0';
    }

    return new ConfigItem(key, from_string, to_string);
}

function JsonConfigItem(key) {
    function from_string(s) {
        return JSON.parse(s);
    }

    function to_string(v) {
        return JSON.stringify(v);
    }

    return new ConfigItem(key, from_string, to_string);
}
