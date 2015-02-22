var Config = require('config').Config;
var util = require('util/misc');

/** @constructor */
function ConfigEditor() {
    this.container = util.fromTemplate('config-editor', {});
    this.select = this.container.getElementsByClassName('config-select')[0];
    this.input = this.container.getElementsByClassName('config-input')[0];

    this.dialog = null;

    var option_map = {}
    var fields = Object.getOwnPropertyNames(Config);
    fields.sort();
    for (var i = 0; i < fields.length; ++i) {
        var field = fields[i];
        var conf = Config[field];

        var option = util.element('option', [
                'value=' + field,
                'text=' + conf.key], this.select);
        if (conf.isSet()) {
            option.classList.add('active');
        }

        option_map[field] = option;
    }

    var this_ = this;
    this.select.onchange = function() {
        this_._handleChange();
    };
    this.container.getElementsByClassName('config-save')[0].onclick = function() {
        this_._doSave();
    };
    this.container.getElementsByClassName('config-reset')[0].onclick = function() {
        this_._doReset();
    };
    this.container.getElementsByClassName('config-close')[0].onclick = function() {
        this_.dialog.hide();
    };
}
exports.ConfigEditor = ConfigEditor;

ConfigEditor.prototype._handleChange = function() {
    var field = this.select.value;
    var conf = Config[field];
    var value = conf.get();
    this.input.value = JSON.stringify(value, null, 4);
};

ConfigEditor.prototype._doSave = function() {
    var field = this.select.value;
    if (!field) {
        return;
    }
    var conf = Config[field];

    try {
        var value = JSON.parse(this.input.value);
    } catch (e) {
        alert('error parsing value: ' + e);
    }

    conf.set(value);

    this._refreshActive();
};

ConfigEditor.prototype._doReset = function() {
    var field = this.select.value;
    if (!field) {
        return;
    }
    var conf = Config[field];
    conf.reset();

    this._refreshActive();
};

ConfigEditor.prototype._refreshActive = function() {
    var option = this.select.selectedOptions[0];
    var field = option.value;
    var conf = Config[field];
    
    if (conf.isSet()) {
        option.classList.add('active');
    } else {
        option.classList.remove('active');
    }

    this._handleChange();
};

ConfigEditor.prototype.handleOpen = function(dialog) {
    this.dialog = dialog;
    dialog.keyboard.pushHandler(function(d, e) {
        // Close on Esc.
        if (d && e.keyCode == 27) {
            dialog.hide();
            return true;
        }
        return false;
    });
};

ConfigEditor.prototype.handleClose = function(dialog) {
    this.dialog = null;
    dialog.keyboard.popHandler();
};
