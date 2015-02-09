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
    if (this._content.handleClose != null) {
        this._content.handleClose(this);
    }

    this._content = null;
    this.inner.removeChild(this.inner.firstChild);
    this.container.classList.add('hidden');
};

Dialog.prototype.show = function(content) {
    this._content = content;
    this.inner.appendChild(content.container);
    this.container.classList.remove('hidden');

    if (this._content.handleOpen != null) {
        this._content.handleOpen(this);
    }
};
