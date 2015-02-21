var util = require('util/misc');

var key_map = {
    37: '&larr;',
    39: '&rarr;',
    38: '&uarr;',
    40: '&darr;',
    16: 'Shift',
    27: 'ESC',
    32: 'Space',
};

function getKeyName(code) {
    if (key_map[code] != null) {
        return key_map[code];
    }
    if (code >= 'A'.charCodeAt(0) && code <= 'Z'.charCodeAt(0)) {
        return String.fromCharCode(code);
    }
    return null;
}


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
    console.log('keydown', evt.keyCode, name);
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
