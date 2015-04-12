var OffscreenContext = require('graphics/canvas').OffscreenContext;
var Program = require('graphics/glutil').Program;
var Buffer = require('graphics/glutil').Buffer;

var GlObject = require('graphics/glutil').GlObject;
var uniform = require('graphics/glutil').uniform;
var attribute = require('graphics/glutil').attribute;


/** @constructor */
function PonyOutline3D(gl, assets) {
    var vert = assets['sprite.vert'];
    var frag = assets['sprite_pony_outline.frag'];
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
    this._obj = new GlObject(gl, program,
            uniforms,
            {'posOffset': attribute(buffer, 2, gl.UNSIGNED_BYTE, false, 0, 0)},
            textures);
}
exports.PonyOutline3D = PonyOutline3D;

PonyOutline3D.prototype.setCamera = function(sx, sy, sw, sh) {
    this._obj.setUniformValue('cameraPos', [sx, sy]);
    this._obj.setUniformValue('cameraSize', [sw, sh]);
};


// Draw the sprite.  It would normally appear at (base_x, base_y) on the
// screen, but it has been clipped to the region defined by clip_* (in
// sprite-relative coordinates).
PonyOutline3D.prototype.draw = function(r, sprite, slice_frac) {
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
        color_arr.push((layer.skip || layer.outline_skip) ? 0 : 1);
        textures['sheetSampler[' + i + ']'] = tex;
    }

    var uniforms = {
        'sheetSize': [tex.width, tex.height],
        'pos': [sprite.ref_x, sprite.ref_y, sprite.ref_z],
        'base': [extra.offset_x + (sprite.flip ? sprite.width : 0),
                 extra.offset_y],
        'size': [(sprite.flip ? -sprite.width : sprite.width),
                 sprite.height],
        'anchor': [sprite.anchor_x, sprite.anchor_y],
        'color': color_arr,
    };

    this._obj.draw(0, 6, uniforms, {}, textures);
};
