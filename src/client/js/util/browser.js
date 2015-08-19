var Config = require('config').Config;
var widget = require('ui/widget');

function isMobile() {
    if (Config.debug_force_mobile_warning.get()) {
        return true;
    }
    var ua = window.navigator.userAgent;
    return (ua.search(/mobi/i) != -1);
}

function isSupported() {
    if (Config.debug_force_browser_warning.get()) {
        return false;
    }
    var ua = window.navigator.userAgent;
    return (ua.search(/(Firefox|Chrome|Chromium)\//) != -1);
}

function checkBrowser(dialog, cb) {
    function handler(e) {
        e.preventDefault();
        e.stopPropagation();
        dialog.hide();
        cb();
        Config.ignore_browser_warning.set(true);
    }

    if (Config.ignore_browser_warning.get()) {
        cb();
    } else if (isMobile()) {
        var div = document.getElementById('unsupported-mobile');
        var try_link = div.getElementsByClassName('unsupported-try')[0];
        try_link.addEventListener('click', handler);

        var f = new widget.Form(new widget.Element(div));
        f.oncancel = function() {};
        dialog.show(f);
    } else if (!isSupported()) {
        var div = document.getElementById('unsupported-browser');
        var try_link = div.getElementsByClassName('unsupported-try')[0];
        try_link.addEventListener('click', handler);

        var f = new widget.Form(new widget.Element(div));
        f.oncancel = function() {};
        dialog.show(f);
    } else {
        cb();
    }
}
exports.checkBrowser = checkBrowser;
