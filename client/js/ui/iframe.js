var Config = require('config').Config;
var widget = require('ui/widget');

/** @constructor */
function Iframe(src) {
    this.dom = document.getElementById('credits');
    this.keys = widget.NULL_KEY_HANDLER;
    this.src = src;
}
exports.Iframe = Iframe;

Iframe.prototype.handleOpen = function(dialog) {
    var this_ = this;
    this.dialog = dialog;
    dialog.keyboard.pushHandler(function(down, evt) {
        if (!down) {
            return false;
        }

        var binding = Config.keybindings.get()[evt.keyCode];
        switch (binding) {
            case 'cancel':
                dialog.hide();
                return true;
        }
        return false;
    });

    this.dom.src = this.src;

    this.dom.onload = function() {
        dialog.keyboard.attach(this.contentDocument);
        this.contentDocument.documentElement.style.fontSize =
            document.documentElement.style.fontSize;

        // TODO: hack to get focus to work.  Firefox doesn't seem to allow
        // changing focus inside a keyboard event handler?
        var this_ = this;
        setTimeout(function() { this_.focus(); }, 0);
    };
};

Iframe.prototype.handleClose = function(dialog) {
    this.dialog = null;
    dialog.keyboard.popHandler();

    this.dom.onload = null;   // In case it hasn't finished loading yet.
    dialog.keyboard.detach(this.dom.contentDocument);

    setTimeout(function() { window.focus(); }, 0);
};
