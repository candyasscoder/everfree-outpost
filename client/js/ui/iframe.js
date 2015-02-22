var Config = require('config').Config;

/** @constructor */
function Iframe(src) {
    this.container = document.getElementById('credits');
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

    this.container.src = this.src;

    this.container.onload = function() {
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

    this.container.onload = null;   // In case it hasn't finished loading yet.
    dialog.keyboard.detach(this.container.contentDocument);

    setTimeout(function() { window.focus(); }, 0);
};
