/** @constructor */
function AnimCanvas(frame_callback, ctx_type) {
    this.canvas = document.createElement('canvas');

    if (ctx_type == null) {
        ctx_type = '2d';
    }

    if (ctx_type == '2d') {
        this.ctx = this.canvas.getContext('2d');
    } else if (ctx_type == 'webgl') {
        var aliases = ['webgl2', 'experimental-webgl2', 'webgl', 'experimental-webgl'];
        this.ctx = null;
        for (var i = 0; i < aliases.length && this.ctx == null; ++i) {
            this.ctx = this.canvas.getContext(aliases[i]);
        }
        if (this.ctx == null) {
            throw 'webgl is not available';
        }
    } else {
        throw 'unknown context type: ' + ctx_type;
    }

    this._handleResize();
    this.animating = false;

    var this_ = this;

    window.addEventListener('resize', function() {
        this_._handleResize();
    });

    function frameWrapper() {
        frame_callback(this_.ctx, Date.now());
        if (this_.animating) {
            window.requestAnimationFrame(frameWrapper);
        }
    }
    // Save frameWrapper for calls to `start()`.
    this._cb = frameWrapper;
}
exports.AnimCanvas = AnimCanvas;

AnimCanvas.prototype.start = function() {
    this.animating = true;
    window.requestAnimationFrame(this._cb);
};

AnimCanvas.prototype.stop = function() {
    this.animating = false;
};

function calcScale(px) {
    var target = 1024;
    if (px < target) {
        return -Math.round(target / px);
    } else {
        return Math.round(px / target);
    }
}

AnimCanvas.prototype._handleResize = function() {
    var width = window.innerWidth;
    var height = window.innerHeight;

    this.scale = calcScale(Math.max(width, height));

    var scale;
    var invScale;
    if (this.scale < 0) {
        invScale = -this.scale;
        scale = 1.0 / invScale;
    } else {
        scale = this.scale;
        invScale = 1.0 / scale;
    }

    var virtWidth = Math.ceil(width * invScale);
    var virtHeight = Math.ceil(height * invScale);

    var physWidth = Math.round(virtWidth * scale);
    var physHeight = Math.round(virtHeight * scale);

    this.canvas.width = virtWidth;
    this.canvas.height = virtHeight;
    this.canvas.style.width = physWidth + 'px';
    this.canvas.style.height = physHeight + 'px';

    // TODO: this is really not an appropriate place to put this code
    var fontSize = 16 * scale;
    document.firstElementChild.style.fontSize = fontSize + 'px';
};


/** @constructor */
function OffscreenContext(width, height) {
    var canvas = document.createElement('canvas');
    canvas.width = width;
    canvas.height = height;
    return canvas.getContext('2d');
}
exports.OffscreenContext = OffscreenContext;
