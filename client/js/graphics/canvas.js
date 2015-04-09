/** @constructor */
function AnimCanvas(frame_callback, ctx_type, webgl_extensions) {
    this.canvas = document.createElement('canvas');

    if (ctx_type == null) {
        ctx_type = '2d';
    }

    if (ctx_type == '2d') {
        this.ctx = this.canvas.getContext('2d');
    } else if (ctx_type == 'webgl') {
        var aliases = ['webgl', 'experimental-webgl'];
        this.ctx = null;
        for (var i = 0; i < aliases.length && this.ctx == null; ++i) {
            this.ctx = this.canvas.getContext(aliases[i]);
        }
        if (this.ctx == null) {
            throw 'webgl is not available';
        }

        this.ext = {};
        if (webgl_extensions != null) {
            for (var i = 0; i < webgl_extensions.length; ++i) {
                var ext_name = webgl_extensions
                var ext_obj = this.ctx.getExtension(ext_name);
                if (ext_obj == null) {
                    throw 'webgl extension ' + ext_name + ' is not available';
                }
                this.ext[ext_name] = ext_obj;
            }
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
        frame_callback(this_, Date.now());
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

    // Make sure phys* is an exact multiple of virt*, even when the actual
    // window size is not.  (Example: 2x scale, non-even window width)
    var physWidth = Math.round(virtWidth * scale);
    var physHeight = Math.round(virtHeight * scale);

    this.canvas.width = physWidth;
    this.canvas.height = physHeight;
    this.canvas.style.width = physWidth + 'px';
    this.canvas.style.height = physHeight + 'px';

    // TODO: this is really not an appropriate place to put this code
    var fontSize = 16 * scale;
    document.firstElementChild.style.fontSize = fontSize + 'px';
    document.body.dataset.scale = scale;

    this.virtualWidth = virtWidth;
    this.virtualHeight = virtHeight;
};


/** @constructor */
function OffscreenContext(width, height) {
    var canvas = document.createElement('canvas');
    canvas.width = width;
    canvas.height = height;
    return canvas.getContext('2d');
}
exports.OffscreenContext = OffscreenContext;
