var Program = require('graphics/glutil').Program;
var Buffer = require('graphics/glutil').Buffer;

var GlObject = require('graphics/glutil').GlObject;
var uniform = require('graphics/glutil').uniform;
var attribute = require('graphics/glutil').attribute;


/** @constructor */
function Simple2D() {
}
exports.Simple2D = Simple2D;

Simple2D.prototype.drawInto = function(ctx, base, sprite) {
    var extra = sprite.extra;

    var x = sprite.ref_x - sprite.anchor_x - base[0];
    var y = sprite.ref_y - sprite.ref_z - sprite.anchor_y - base[1];

    if (sprite.flip) {
        ctx.scale(-1, 1);
        x = -x - sprite.width;
    }

    ctx.drawImage(extra.image,
            extra.offset_x,
            extra.offset_y,
            sprite.width,
            sprite.height,
            x,
            y,
            sprite.width,
            sprite.height);

    if (sprite.flip) {
        ctx.scale(-1, 1);
    }
};


/** @constructor */
function Simple3D(gl, assets) {
    this.gl = gl;

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
        'sheetSize': uniform('vec2', null),
        'srcPos': uniform('vec2', null),
        'srcSize': uniform('vec2', null),
        'destPos': uniform('vec2', null),
        'destSize': uniform('vec2', null),
    };
    this._obj = new GlObject(gl, program,
            uniforms,
            {'position': attribute(buffer, 2, gl.UNSIGNED_BYTE, false, 0, 0)},
            {'sheetSampler': null});
}
exports.Simple3D = Simple3D;

Simple3D.prototype.setCamera = function(sx, sy, sw, sh) {
    this._obj.setUniformValue('cameraPos', [sx, sy]);
    this._obj.setUniformValue('cameraSize', [sw, sh]);
};

Simple3D.prototype.draw = function(r, sprite, base_x, base_y, clip_x, clip_y, clip_w, clip_h) {
    var extra = sprite.extra;
    var tex = r.cacheTexture(extra.image);

    var off_x = clip_x;
    var off_y = clip_y;
    if (sprite.flip) {
        off_x = sprite.width - off_x;
    }

    var uniforms = {
        'srcPos':       [extra.offset_x + off_x,
                         extra.offset_y + off_y],
        'srcSize':      [(sprite.flip ? -clip_w : clip_w),
                         clip_h],
        'destPos':      [base_x + clip_x,
                         base_y + clip_y],
        'destSize':     [clip_w,
                         clip_h],

        'sheetSize': [tex.width, tex.height],
    };

    var textures = {
        'sheetSampler': tex,
    };

    this._obj.draw(0, 6, uniforms, {}, textures);
};


/** @constructor */
function SimpleExtra(image) {
    this.image = image;
    this.offset_x = 0;
    this.offset_y = 0;
}
exports.SimpleExtra = SimpleExtra;

SimpleExtra.prototype.getClass = function() {
    return 'simple';
};

SimpleExtra.prototype.updateIJ = function(sprite, i, j) {
    this.offset_x = j * sprite.width;
    this.offset_y = i * sprite.height;
};
