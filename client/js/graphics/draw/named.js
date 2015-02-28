var OffscreenContext = require('graphics/canvas').OffscreenContext;
var Program = require('graphics/glutil').Program;
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


/** @constructor */
function NameBuffer(assets) {
    this.ctx = new OffscreenContext(NAME_BUFFER_WIDTH, NAME_BUFFER_HEIGHT);
    this.cache = new StringCache(NAME_BUFFER_COUNT);

    this.font = new Font(assets['font'], assets['font_metrics']);
}

NameBuffer.prototype._draw = function(s, idx) {
    var x = NAME_WIDTH * (idx % NAME_BUFFER_COUNT_X);
    var y = NAME_HEIGHT * ((idx / NAME_BUFFER_COUNT_Y)|0);
    var ctx = this.ctx;

    var str_width = this.font.measureWidth(s);
    var offset_x = Math.floor((NAME_WIDTH - str_width) / 2);

    ctx.save();

    ctx.clearRect(x, y, NAME_WIDTH, NAME_HEIGHT);
    console.log('clip to ', x, y, NAME_WIDTH, NAME_HEIGHT);
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
    var y = NAME_HEIGHT * ((idx / NAME_BUFFER_COUNT_Y)|0);
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

        console.log('draw', s.charCodeAt(i), code, idx, x, y);

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
    var program = new Program(gl, vert, frag);

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
        'srcPos': uniform('vec2', null),
        'srcSize': uniform('vec2', null),
        'destPos': uniform('vec2', null),
        'destSize': uniform('vec2', null),
    };

    this._texture = new Texture(gl);
    this._refreshTexture();
    this._name_obj = new GlObject(gl, program,
            uniforms,
            {'position': attribute(buffer, 2, gl.UNSIGNED_BYTE, false, 0, 0)},
            {'sheetSampler': this._texture});
}
exports.Named3D = Named3D;

Named3D.prototype._refreshTexture = function() {
    this._texture.loadImage(this._names.image());
};

Named3D.prototype.setCamera = function(sx, sy, sw, sh) {
    this.layered.setCamera(sx, sy, sw, sh);
    this._name_obj.setUniformValue('cameraPos', [sx, sy]);
    this._name_obj.setUniformValue('cameraSize', [sw, sh]);
};

Named3D.prototype.draw = function(r, sprite, base_x, base_y, clip_x, clip_y, clip_w, clip_h) {
    this.layered.draw(r, sprite, base_x, base_y, clip_x, clip_y, clip_w, clip_h);

    // TODO: hardcoded name positioning, should be computed somehow to center
    // the name at a reasonable height.
    var x = 0;
    var y = 10;
    var w = NAME_WIDTH;
    var h = NAME_HEIGHT;

    var name_x = Math.max(x, clip_x);
    var name_y = Math.max(y, clip_y);
    var name_w = Math.min(x + w, clip_x + clip_w) - name_x;
    var name_h = Math.min(y + h, clip_y + clip_h) - name_y;

    if (name_w <= 0 || name_h <= 0) {
        // Name region does not overlap clip region.
        return;
    }

    var off = this._names.offset(sprite.extra.name);
    if (off.created) {
        this._refreshTexture();
    }

    var uniforms = {
        'srcPos':       [name_x - x + off.x,
                         name_y - y + off.y],
        'srcSize':      [name_w,
                         name_h],
        'destPos':      [base_x + name_x,
                         base_y + name_y],
        'destSize':     [name_w,
                         name_h],
    };
    this._name_obj.draw(0, 6, uniforms, {}, {});
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
