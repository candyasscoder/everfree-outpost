var DEFAULT_CONFIG = {
    'show_controls': true,
};

/** @constructor */
function Config() {
    this.show_controls = new BooleanConfigItem('show_controls');
}
exports.Config = Config;

/** @constructor */
function ConfigItem(key, from, to) {
    this.key = key;
    this.from_string = from;
    this.to_string = to;
}

ConfigItem.prototype.get = function() {
    var value = localStorage.getItem(this.key);
    if (!value) {
        return DEFAULT_CONFIG[this.key];
    } else {
        return this.from_string(value);
    }
};

ConfigItem.prototype.set = function(value) {
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
