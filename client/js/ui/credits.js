var Config = require('config').Config;

/** @constructor */
function Credits() {
    this.container = document.getElementById('credits');
}
exports.Credits = Credits;

Credits.prototype.handleOpen = function(dialog) {
    var this_ = this;
    this.dialog = dialog;
    dialog.keyboard.pushHandler(function(down, evt) {
        if (!down) {
            return false;
        }

        var binding = Config.keybindings.get()[evt.keyCode];
        switch (binding) {
            case 'cancel':
            case 'show_credits':
                dialog.hide();
                return true;
        }
        return false;
    });

    this.container.src = 'credits.html';
};

Credits.prototype.handleClose = function(dialog) {
    this.dialog = null;
    dialog.keyboard.popHandler();
};
