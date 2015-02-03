/** @constructor */
function Dialog() {
    this.container = document.createElement('div');
    this.container.classList.add('dialog-container');

    this.inner = document.createElement('div');
    this.inner.classList.add('dialog');
    this.container.appendChild(this.inner);

    this._keyboard = null;

    this.hide();
}
exports.Dialog = Dialog;

Dialog.prototype.hide = function() {
    if (this._keyboard != null) {
        this._keyboard.popHandler();
        this._keyboard = null;
    }

    if (this.inner.firstChild != null) {
        this.inner.removeChild(this.inner.firstChild);
    }
    this.container.style.display = 'none';
}

Dialog.prototype.show = function(content, keyboard, handler) {
    if (keyboard != null) {
        console.assert(this._keyboard == null);
        keyboard.pushHandler(handler);
        this._keyboard = keyboard;
    }

    if (this.inner.firstChild != null) {
        this.inner.removeChild(this.inner.firstChild);
    }
    this.inner.appendChild(content);
    this.container.style.display = 'flex';
}
