var util = require('util/misc');
var getKeyName = require('util/keynames').getKeyName;


/** @constructor */
function KeyDisplay() {
    this.container = document.getElementById('key-display');
}
exports.KeyDisplay = KeyDisplay;

KeyDisplay.prototype.onKeyDown = function(evt) {
    if (evt.repeat) {
        return;
    }

    var name = getKeyName(evt.keyCode);
    if (name == null) {
        return;
    }

    var new_key = util.element('kbd');
    new_key.innerHTML = name;
    new_key.dataset.code = evt.keyCode;

    for (var cur = this.container.firstElementChild; cur != null; cur = cur.nextElementSibling) {
        if (cur.dataset.code >= evt.keyCode) {
            this.container.insertBefore(new_key, cur);
            return;
        }
    }
    this.container.appendChild(new_key);
};

KeyDisplay.prototype.onKeyUp = function(evt) {
    var cur = this.container.firstElementChild;
    while (cur != null) {
        var next = cur.nextElementSibling;
        if (cur.dataset.code == evt.keyCode) {
            this.container.removeChild(cur);
        }
        cur = next;
    }
};
