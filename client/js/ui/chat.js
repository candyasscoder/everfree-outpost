var util = require('util/misc');
var Config = require('config').Config;

/** @constructor */
function ChatWindow() {
    this.container = util.element('div', ['chat-container']);
    this._content = util.element('div', ['chat'], this.container);
    this._entry = util.element('input', ['chat-input'], this.container);
    this._entry.disabled = true;

    this.count = 0;
}
exports.ChatWindow = ChatWindow;

ChatWindow.prototype.addMessage = function(msg) {
    var idx = msg.indexOf('\t');
    if (idx == -1) {
        console.assert(false, 'msg is missing delimiter', msg);
        return;
    }

    var name = msg.substr(0, idx);
    var text = msg.substr(idx + 1);

    var lineDiv = util.element('div', ['chat-line'], this._content);
    var nameDiv = util.element('div', ['chat-name'], lineDiv);
    var textDiv = util.element('div', ['chat-text'], lineDiv);
    nameDiv.textContent = name;
    textDiv.textContent = text;

    if (name == '***') {
        lineDiv.classList.add('server-message');
    }

    var limit = Config.chat_scrollback.get();
    if (this.count < limit) {
        this.count += 1;
    } else {
        this._content.removeChild(this._content.firstChild);
    }

    this._content.scrollTop = this._content.scrollHeight;
};

ChatWindow.prototype.startTyping = function(keyboard, conn, init) {
    var this_ = this;

    this._entry.disabled = false;
    this._entry.value = init;
    this._entry.focus();

    keyboard.pushHandler(function(down, evt) {
        if (document.activeElement !== this_._entry) {
            this_._entry.focus();
        }
        if (!down) {
            return false;
        }

        var binding = Config.chat_keybindings.get()[evt.keyCode];

        switch (binding) {
            case 'send':
                this_.finishTyping(keyboard, conn, true);
                return true;
            case 'cancel':
                this_.finishTyping(keyboard, conn, false);
                return true;
            default:
                return false;
        }
    });
};

ChatWindow.prototype.finishTyping = function(keyboard, conn, send) {
    keyboard.popHandler();

    if (send && this._entry.value != '') {
        conn.sendChat(this._entry.value);
    }

    this._entry.blur();
    this._entry.value = '';
    this._entry.disabled = true;
};
