var Config = require('config').Config;


/** @constructor */
function DNDState(keyboard) {
    this.keyboard = keyboard;

    this.data = null;
    this.source = null;
    this.icon = null;

    var this_ = this;
    this.move_listener = function(evt) { this_._handleMove(evt); };
    this.click_listener = function(evt) { this_._handleClick(evt); };
    this.key_handler = function(down, evt) { this_._handleKey(down, evt); };
    this.contextmenu_listener = function(evt) { evt.preventDefault(); };
}
exports.DNDState = DNDState;

DNDState.prototype.registerSource = function(source) {
    var this_ = this;
    source.dom.addEventListener('click', function(evt) {
        if (this_.data != null) {
            return;
        }

        var data = source.ondragstart(evt);
        if (data != null) {
            evt.preventDefault();
            evt.stopPropagation();
            // Don't instantly fire a drop on this same element.
            evt.stopImmediatePropagation();
            this_._startDrag(source, data, evt);
        }
    });
};

DNDState.prototype.registerDest = function(dest) {
    var this_ = this;
    dest.dom.addEventListener('click', function(evt) {
        if (this_.data != null && (dest.candrop == null || dest.candrop(this_.data))) {
            evt.preventDefault();
            evt.stopPropagation();
            this_._finishDrag(dest);
        }
    });
};

DNDState.prototype.cancelDrag = function(source) {
    if (this.source !== source) {
        return;
    }

    this._cancelDrag();
};


DNDState.prototype._startDrag = function(source, data, evt) {
    if (this.source != null) {
        return;
    }

    this.source = source;
    this.data = data;
    this.icon = data.icon || null;

    document.addEventListener('mousemove', this.move_listener);
    document.addEventListener('click', this.click_listener);
    document.addEventListener('contextmenu', this.contextmenu_listener);
    this.keyboard.pushHandler(this.key_handler);
    this._addIcon(evt);
};

DNDState.prototype._endDrag = function() {
    document.removeEventListener('mousemove', this.move_listener);
    document.removeEventListener('click', this.click_listener);
    document.removeEventListener('contextmenu', this.contextmenu_listener);
    this.keyboard.popHandler();
    this._removeIcon();

    this.source = null;
    this.data = null;
    this.icon = null;
};

DNDState.prototype._finishDrag = function(dest) {
    if (this.source.ondragfinish != null) {
        this.source.ondragfinish(dest, this.data);
    }
    this._endDrag();
};

DNDState.prototype._cancelDrag = function() {
    if (this.source.ondragcancel != null) {
        this.source.ondragcancel(this.data);
    }
    this._endDrag();
};


DNDState.prototype._handleMove = function(evt) {
    this._updateIcon(evt);
};

DNDState.prototype._handleClick = function(evt) {
    if (evt.button != 0) {
        this._cancelDrag();
    }
};

DNDState.prototype._handleKey = function(down, evt) {
    if (!down) {
        return;
    }

    var binding = Config.ui_keybindings.get()[evt.keyCode];
    if (binding == 'cancel') {
        this._cancelDrag();
        return true;
    }
};


DNDState.prototype._addIcon = function(evt) {
    if (this.icon == null) {
        return;
    }

    document.body.appendChild(this.icon);
    this.icon.style.position = 'absolute';
    var scale = document.body.dataset.uiScale;
    this.icon.style.transform = 'scale(' + scale + ')';
    this.icon.style.transformOrigin = 'left top';
    this._updateIcon(evt);
};

DNDState.prototype._updateIcon = function(evt) {
    if (this.icon == null) {
        return;
    }

    this.icon.style.left = (evt.clientX + 10) + 'px';
    this.icon.style.top = (evt.clientY + 10) + 'px';
};

DNDState.prototype._removeIcon = function(evt) {
    if (this.icon == null) {
        return;
    }

    document.body.removeChild(this.icon);
};
