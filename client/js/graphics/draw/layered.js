var OffscreenContext = require('graphics/canvas').OffscreenContext;
var buildPrograms = require('graphics/glutil').buildPrograms;
var Buffer = require('graphics/glutil').Buffer;

var GlObject = require('graphics/glutil').GlObject;
var uniform = require('graphics/glutil').uniform;
var attribute = require('graphics/glutil').attribute;


/** @constructor */
function Layered2D() {
}
exports.Layered2D = Layered2D;

Layered2D.prototype.drawInto = function(ctx, base, sprite) {
    var extra = sprite.extra;

    var x = sprite.ref_x - sprite.anchor_x - base[0];
    var y = sprite.ref_y - sprite.ref_z - sprite.anchor_y - base[1];
    var w = sprite.width;
    var h = sprite.height;

    var buf = new OffscreenContext(w, h);
    var buf_x = x;
    var buf_y = 0;

    if (sprite.flip) {
        buf.scale(-1, 1);
        buf_x = -buf_x - w;
    }

    var off_x = extra.offset_x;
    var off_y = extra.offset_y;

    for (var i = 0; i < extra.layers.length; ++i) {
        var layer = extra.layers[i];
        if (layer.skip) {
            continue;
        }

        buf.globalCompositeOperation = 'copy';
        buf.drawImage(layer.image,
                off_x, off_y, w, h,
                buf_x, buf_y, w, h);

        buf.globalCompositeOperation = 'source-in';
        buf.fillStyle = colorString(layer.color);
        buf.fillRect(buf_x, buf_y, w, h);

        buf.globalCompositeOperation = 'multiply';
        buf.drawImage(layer.image,
                off_x, off_y, w, h,
                buf_x, buf_y, w, h);

        ctx.drawImage(buf.canvas, x, y);
    }
};

function colorString(color) {
    var str = color.toString(16);
    while (str.length < 6) {
        str = '0' + str;
    }
    return '#' + str;
}


/** @constructor */
function Layered3D(gl, assets) {
    var vert = assets['sprite.vert'];
    var frag = assets['sprite_layered.frag'];
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
        'sheetSize': uniform('vec2', null),
        'sliceFrac': uniform('float', null),
        'pos': uniform('vec3', null),
        'base': uniform('vec2', null),
        'size': uniform('vec2', null),
        'anchor': uniform('vec2', null),
        'color': uniform('vec4', null),
    };
    var textures = {};
    for (var i = 0; i < 8; ++i) {
        textures['sheetSampler[' + i + ']'] = null;
    }
    this._obj = new GlObject(gl, programs,
            uniforms,
            {'posOffset': attribute(buffer, 2, gl.UNSIGNED_BYTE, false, 0, 0)},
            textures);
}
exports.Layered3D = Layered3D;

Layered3D.prototype.setCamera = function(sx, sy, sw, sh) {
    this._obj.setUniformValue('cameraPos', [sx, sy]);
    this._obj.setUniformValue('cameraSize', [sw, sh]);
};


// Draw the sprite.  It would normally appear at (base_x, base_y) on the
// screen, but it has been clipped to the region defined by clip_* (in
// sprite-relative coordinates).
Layered3D.prototype.draw = function(fb_idx, r, sprite, slice_frac) {
    var extra = sprite.extra;
    var textures = {};
    var color_arr = [];

    for (var i = 0; i < extra.layers.length; ++i) {
        var layer = extra.layers[i];
        var tex = r.cacheTexture(layer.image);

        var color_int = layer.color;
        color_arr.push(((color_int >> 16) & 0xff) / 255.0);
        color_arr.push(((color_int >>  8) & 0xff) / 255.0);
        color_arr.push(((color_int)       & 0xff) / 255.0);
        color_arr.push(layer.skip ? 0 : 1);
        textures['sheetSampler[' + i + ']'] = tex;
    }

    var uniforms = {
        'sheetSize': [tex.width, tex.height],
        'sliceFrac': [slice_frac],
        'pos': [sprite.ref_x, sprite.ref_y, sprite.ref_z],
        'base': [extra.offset_x + (sprite.flip ? sprite.width : 0),
                 extra.offset_y],
        'size': [(sprite.flip ? -sprite.width : sprite.width),
                 sprite.height],
        'anchor': [sprite.anchor_x, sprite.anchor_y],
        'color': color_arr,
    };

    this._obj.draw(fb_idx, 0, 6, uniforms, {}, textures);
};


/** @constructor */
function LayeredExtra(layers) {
    this.layers = layers;
    this.offset_x = 0;
    this.offset_y = 0;
}
exports.LayeredExtra = LayeredExtra;

LayeredExtra.prototype.getClass = function() {
    return 'layered';
};

LayeredExtra.prototype.updateIJ = function(sprite, i, j) {
    this.offset_x = j * sprite.width;
    this.offset_y = i * sprite.height;
};
