var Config = require('config').Config;


/** @constructor */
function Dialog(keyboard) {
    this.container = document.createElement('div');
    this.container.classList.add('dialog-container');
    this.container.classList.add('hidden');

    this.inner = document.createElement('div');
    this.inner.classList.add('dialog');
    this.container.appendChild(this.inner);

    this.keyboard = keyboard;
    this._content = null;
}
exports.Dialog = Dialog;

Dialog.prototype.hide = function() {
    this._content = null;
    this.inner.removeChild(this.inner.firstChild);
    this.container.classList.add('hidden');
    this.keyboard.popHandler();
};

Dialog.prototype.show = function(content) {
    if (this._content != null) {
        this.hide();
    }

    this._content = content;
    this.inner.appendChild(content.dom);
    this.container.classList.remove('hidden');

    var this_ = this;
    this.keyboard.pushHandler(function(down, evt) {
        return this_._content.onkey(new widget.WidgetKeyEvent(down, evt));
    });

    if (this._content.oncancel == null) {
        this._content.oncancel = function() { this_.hide(); };
    }
};
