var util = require('util/misc');
var Config = require('config').Config;

/** @constructor */
function ChatWindow() {
    var lines = Config.chat_lines.get();
    // Font size is 0.7rem.  Add a little bit extra to cover line spacing.
    var height = (lines * 0.85) + 'rem';

    var parts = util.templateParts('chat-panel');
    this.container = parts['top'];
    this._content = parts['content'];
    this._content.style.heigh = height;
    this._entry = parts['entry'];

    if (Config.chat_autohide.get()) {
        this.container.style.display = 'none';
    }

    this.count = 0;
}
exports.ChatWindow = ChatWindow;

ChatWindow.prototype.addMessage = function(msg) {
    var idx = msg.indexOf('\t');
    if (idx == -1) {
        console.assert(false, 'msg is missing delimiter', msg);
        return;
    }

    var name = msg.substring(0, idx);
    if (Config.ignores.get()[name]) {
        return;
    }
    var text = msg.substring(idx + 1);

    var parts = util.templateParts('chat-line');
    parts['name'].textContent = name;
    parts['text'].textContent = text;

    if (name == '***') {
        parts['top'].classList.add('server-message');
    }

    this._content.appendChild(parts['top']);

    var limit = Config.chat_scrollback.get();
    if (this.count < limit) {
        this.count += 1;
    } else {
        this._content.removeChild(this._content.firstChild);
    }

    this._content.scrollTop = this._content.scrollHeight;
};

ChatWindow.prototype.addIgnore = function(name) {
    var ignores = Config.ignores.get();
    ignores['<' + name + '>'] = true;
    Config.ignores.save();
};

ChatWindow.prototype.removeIgnore = function(name) {
    var ignores = Config.ignores.get();
    delete ignores['<' + name + '>'];
    Config.ignores.save();
};

ChatWindow.prototype.startTyping = function(keyboard, conn, init) {
    var this_ = this;

    if (Config.chat_autohide.get()) {
        this.container.style.display = 'flex';
    }

    this._entry.disabled = false;
    this._entry.value = init;
    this._entry.focus();
    this._entry.selectionStart = init.length;

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

    var msg = this._entry.value;
    var handled = false;
    if (msg[0] == '/') {
        var idx = msg.indexOf(' ');
        if (idx != -1) {
            var cmd = msg.substring(1, idx);
            var arg = msg.substring(idx + 1);
            if (cmd == 'ignore') {
                this.addIgnore(arg);
                handled = true;
            } else if (cmd == 'unignore') {
                this.removeIgnore(arg);
                handled = true;
            }
        }
    }

    if (send && !handled && msg != '') {
        conn.sendChat(msg);
    }

    this._entry.blur();
    this._entry.value = '';
    this._entry.disabled = true;

    var this_ = this;
    if (Config.chat_autohide.get()) {
        window.setTimeout(function() {
            if (!this_._entry.disabled) {
                // User already started typing again.
                return;
            }
            this_.container.style.display = 'none';
        }, 3000);
    }
};
