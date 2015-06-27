var KEY_MAP = (function() {
    var map = {
        37: '&larr;',
        39: '&rarr;',
        38: '&uarr;',
        40: '&darr;',

        16: 'Shift',
        17: 'Ctrl',
        18: 'Alt',

        13: 'Enter',
        32: 'Space',
         9: 'Tab',
         8: 'Bksp',
        27: 'Esc',

       191: '/',
    };

    // F1-F12
    for (var i = 0; i < 12; ++i) {
        map[112 + i] = 'F' + (i + 1);
    }

    // Letters
    for (var i = 0; i < 26; ++i) {
        var code = 0x41 + i;
        map[code] = String.fromCharCode(code);
    }

    // Digits
    for (var i = 0; i < 10; ++i) {
        var code = 0x30 + i;
        map[code] = String.fromCharCode(code);
    }

    return map;
})();

function getKeyName(code) {
    var name = KEY_MAP[code];
    if (name != null) {
        return name;
    } else {
        return '#' + code;
    }
}
exports.getKeyName = getKeyName;
