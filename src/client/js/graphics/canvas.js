var Config = require('config').Config;


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
                var ext_name = webgl_extensions[i];
                if (Config.debug_block_webgl_extensions.get()[ext_name]) {
                    continue;
                }
                var ext_obj = this.ctx.getExtension(ext_name);
                this.ext[ext_name] = ext_obj;
            }
        }
    } else {
        throw 'unknown context type: ' + ctx_type;
    }

    this.animating = false;

    var this_ = this;

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

AnimCanvas.prototype.resize = function(phys_w, phys_h, virt_w, virt_h) {
    this.canvas.width = phys_w;
    this.canvas.height = phys_h;
    this.canvas.style.width = phys_w + 'px';
    this.canvas.style.height = phys_h + 'px';

    this.virtualWidth = virt_w;
    this.virtualHeight = virt_h;
};


/** @constructor */
function OffscreenContext(width, height) {
    var canvas = document.createElement('canvas');
    canvas.width = width;
    canvas.height = height;
    return canvas.getContext('2d');
}
exports.OffscreenContext = OffscreenContext;
