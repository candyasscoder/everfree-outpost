var Config = require('config').Config;
var widget = require('ui/widget');

/** @constructor */
function Iframe(src, keyboard) {
    var iframe = util.element('iframe', ['src=' + src]);
    this.dom = iframe;

    iframe.onload = function() {
        var cd = iframe.contentDocument;
        keyboard.attach(cd);

        cd.documentElement.style.fontSize =
            document.documentElement.style.fontSize;
    };
}
exports.Iframe = Iframe;

Iframe.prototype.onkey = function(evt) {
    if (evt.raw.keyCode == 32) {
        evt.requestDefault();
        return true;
    }
};

Iframe.prototype.onfocus = function() {
    this.dom.focus();
};

Iframe.prototype.onblur = function() {
    window.focus();
};
