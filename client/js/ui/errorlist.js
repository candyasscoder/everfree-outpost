var util = require('util/misc');
var ToastList = require('ui/toast').ToastList;


/** @constructor */
function ErrorList() {
    this.toast = new ToastList('error-list', 20, 10000);
    this.container = this.toast.dom;
}
exports.ErrorList = ErrorList;

ErrorList.prototype.attach = function(w) {
    var this_ = this;
    w.onerror = function(msg, url, line, col, err) {
        var last_slash = url.lastIndexOf('/');
        var text = [url.substr(last_slash + 1), line, col, ' '].join(':') + msg;
        var dom = util.element('div', ['text=' + text]);
        this_.toast.add({ dom: dom });
    };
};
