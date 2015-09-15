var Config = require('config').Config;
var widget = require('ui/widget');
var util = require('util/misc');


/** @constructor */
function Dialog(keyboard) {
    var parts = util.templateParts('dialog-container');
    this.container = parts['top'];
    this.inner = parts['inner'];
    this.title = parts['title'];

    this.keyboard = keyboard;
    this._content = null;
}
exports.Dialog = Dialog;

Dialog.prototype.isVisible = function() {
    return (this._content != null);
};

Dialog.prototype.hide = function() {
    var old_content = this._content;
    setTimeout(function() {
        if (old_content.onblur != null) {
            old_content.onblur();
        }
    }, 0);

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
    this.title.textContent = content.dom.dataset['dialogTitle'] || 'Dialog';
    this.container.classList.remove('hidden');

    var this_ = this;
    this.keyboard.pushHandler(function(down, evt) {
        var widget_evt = new widget.WidgetKeyEvent(down, evt);
        var handled = this_._content.onkey(widget_evt)
        return handled && !widget_evt.useDefault;
    });

    if (this._content.oncancel == null) {
        this._content.oncancel = function() { this_.hide(); };
    }

    setTimeout(function() {
        if (this_._content.onfocus != null) {
            this_._content.onfocus();
        }
    }, 0);
};
