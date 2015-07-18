var Config = require('config').Config;

var OffscreenContext = require('graphics/canvas').OffscreenContext;
var buildPrograms = require('graphics/glutil').buildPrograms;
var Buffer = require('graphics/glutil').Buffer;
var Texture = require('graphics/glutil').Texture;

var GlObject = require('graphics/glutil').GlObject;
var uniform = require('graphics/glutil').uniform;
var attribute = require('graphics/glutil').attribute;

var Layered3D = require('graphics/draw/layered').Layered3D;

var StringCache = require('util/stringcache').StringCache;


var NAME_WIDTH = 96;
var NAME_HEIGHT = 12;
var NAME_BUFFER_WIDTH = 512;
var NAME_BUFFER_HEIGHT = 512;
var NAME_BUFFER_COUNT_X = (NAME_BUFFER_WIDTH / NAME_WIDTH)|0;
var NAME_BUFFER_COUNT_Y = (NAME_BUFFER_HEIGHT / NAME_HEIGHT)|0;
var NAME_BUFFER_COUNT = NAME_BUFFER_COUNT_X * NAME_BUFFER_COUNT_Y;

exports.NAME_WIDTH = NAME_WIDTH;
exports.NAME_HEIGHT = NAME_HEIGHT;
exports.NAME_BUFFER_WIDTH = NAME_BUFFER_WIDTH;
exports.NAME_BUFFER_HEIGHT = NAME_BUFFER_HEIGHT;


/** @constructor */
function NameBuffer(assets) {
    this.ctx = new OffscreenContext(NAME_BUFFER_WIDTH, NAME_BUFFER_HEIGHT);
    this.cache = new StringCache(NAME_BUFFER_COUNT);

    this.font = new Font(assets['font'], assets['font_metrics']);
}
exports.NameBuffer = NameBuffer;

NameBuffer.prototype._draw = function(s, idx) {
    var x = NAME_WIDTH * (idx % NAME_BUFFER_COUNT_X);
    var y = NAME_HEIGHT * ((idx / NAME_BUFFER_COUNT_X)|0);
    var ctx = this.ctx;

    var str_width = this.font.measureWidth(s);
    var offset_x = Math.floor((NAME_WIDTH - str_width) / 2);

    ctx.save();

    ctx.clearRect(x, y, NAME_WIDTH, NAME_HEIGHT);
    ctx.rect(x, y, NAME_WIDTH, NAME_HEIGHT);
    ctx.clip();
    this.font.drawString(ctx, s, x + offset_x, y);

    ctx.restore();
};

NameBuffer.prototype.offset = function(s) {
    var idx = this.cache.get(s);
    var created = false;
    if (idx == null) {
        idx = this.cache.put(s);
        this._draw(s, idx);
        created = true;
    }

    var x = NAME_WIDTH * (idx % NAME_BUFFER_COUNT_X);
    var y = NAME_HEIGHT * ((idx / NAME_BUFFER_COUNT_X)|0);
    return { x: x, y: y, created: created };
};

NameBuffer.prototype.image = function() {
    return this.ctx.canvas;
};


/** @constructor */
function Font(image, metrics) {
    this.image = image;

    this.first_char = metrics['first_char'];
    this.xs = metrics['xs'];
    this.y = metrics['y'];
    this.widths = metrics['widths'];
    this.height = metrics['height'];
    this.spacing = metrics['spacing'];
    this.space_width = metrics['space_width'];
}

Font.prototype.getHeight = function() {
    return this.height;
};

Font.prototype.measureWidth = function(s) {
    var total = 0;
    for (var i = 0; i < s.length; ++i) {
        var code = s.charCodeAt(i);
        var idx = code - this.first_char;

        var width;
        if (code == 0x20) {
            width = this.space_width;
        } else {
            width = this.widths[idx] || 0;
        }

        total += width;
        if (i > 0) {
            total += this.spacing;
        }
    }
    return total;
};

Font.prototype.drawString = function(ctx, s, x, y) {
    var h = this.getHeight();

    for (var i = 0; i < s.length; ++i) {
        var code = s.charCodeAt(i);
        var idx = code - this.first_char;

        if (code == 0x20) {
            x += this.space_width;
            continue;
        } else if (idx < 0 || idx >= this.widths.length) {
            // Invalid character
            continue;
        }

        var src_x = this.xs[idx];
        var src_y = this.y;
        var w = this.widths[idx];

        ctx.drawImage(this.image,
                src_x, src_y, w, h,
                x, y, w, h);
        x += w + this.spacing;
    }
};


/** @constructor */
function Named3D(gl, assets) {
    this.layered = new Layered3D(gl, assets);
    this._names = new NameBuffer(assets);

    var vert = assets['sprite.vert'];
    var frag = assets['sprite.frag'];
    var programs = buildPrograms(gl, vert, frag, 2);

    var buffer = new Buffer(gl);
    buffer.loadData(new Uint8Array([
            0, 0,
            0, 1,
            1, 1,

            0, 0,
            1, 1,
            1, 0,
    ]));

    var uniforms = {
        'cameraPos': uniform('vec2', null),
        'cameraSize': uniform('vec2', null),
        'sheetSize': uniform('vec2', [NAME_BUFFER_WIDTH, NAME_BUFFER_HEIGHT]),
        'sliceFrac': uniform('float', null),
        'pos': uniform('vec3', null),
        'base': uniform('vec2', null),
        'size': uniform('vec2', null),
        'anchor': uniform('vec2', null),
    };

    this._texture = new Texture(gl);
    this._refreshTexture();
    this._name_obj = new GlObject(gl, programs,
            uniforms,
            {'posOffset': attribute(buffer, 2, gl.UNSIGNED_BYTE, false, 0, 0)},
            {'sheetSampler': this._texture});
}
exports.Named3D = Named3D;

Named3D.prototype._refreshTexture = function() {
    this._texture.loadImage(this._names.image());
};

Named3D.prototype.setCamera = function(pos, size) {
    this.layered.setCamera(pos, size);
    this._name_obj.setUniformValue('cameraPos', pos);
    this._name_obj.setUniformValue('cameraSize', size);
};

Named3D.prototype.draw = function(fb_idx, r, sprite, slice_frac) {
    this.layered.draw(fb_idx, r, sprite, slice_frac);

    if (!Config.render_names.get()) {
        return;
    }

    var off = this._names.offset(sprite.extra.name);
    if (off.created) {
        this._refreshTexture();
    }

    var uniforms = {
        'sliceFrac': [slice_frac],
        // TODO: hardcoded name positioning, should be computed somehow to
        // center the name at a reasonable height.
        'pos': [sprite.ref_x, sprite.ref_y, sprite.ref_z + 90 - 22],
        'base': [off.x, off.y],
        'size': [NAME_WIDTH, NAME_HEIGHT],
        'anchor': [NAME_WIDTH / 2, NAME_HEIGHT],
    };
    this._name_obj.draw(fb_idx, 0, 6, uniforms, {}, {});
};


/** @constructor */
function NamedExtra(layers, name) {
    this.layers = layers;
    this.offset_x = 0;
    this.offset_y = 0;
    this.name = name;
}
exports.NamedExtra = NamedExtra;

NamedExtra.prototype.getClass = function() {
    return 'named';
};

NamedExtra.prototype.updateIJ = function(sprite, i, j) {
    this.offset_x = j * sprite.width;
    this.offset_y = i * sprite.height;
};
