console.log('going');

var TRIBE_NAME = ['E', 'P', 'U', 'A'];
var COLOR_RAMP = [0x44, 0x88, 0xcc, 0xff];

/** @constructor */
function PonyAppearance(assets, bits, name) {
    var tribe = (bits >> 6) & 3;
    // TODO: use a SpriteSheet object that contains all the sheet images
    this.base_img = assets['pony_f_' + TRIBE_NAME[tribe] + '-0'];

    var r = (bits >> 4) & 3;
    var g = (bits >> 2) & 3;
    var b = (bits >> 0) & 3;
    this.hair_color = [COLOR_RAMP[r + 1], COLOR_RAMP[g + 1], COLOR_RAMP[b + 1]];
    this.body_color = [COLOR_RAMP[r], COLOR_RAMP[g], COLOR_RAMP[b]];
}
exports.PonyAppearance = PonyAppearance;

PonyAppearance.prototype.buildSprite = function(pos, frame) {
    return new Sprite(this)
        .setSize(96, 96)
        .setRefPosition(pos.x, pos.y, pos.z)
        .setAnchor(48, 90)
        .setFrame(frame.sheet, frame.i, frame.j)
        .setFlip(frame.flip);
};

// TODO: move sliceFrac argument into Class.getCamera
PonyAppearance.prototype.draw3D = function(fb_idx, r, sprite, slice_frac) {
    var base_tex = r.cacheTexture(this.base_img);
    var textures = {
        'sheetSampler[0]': base_tex,
    };

    var offset_x = sprite.frame_j * sprite.width;
    var offset_y = sprite.frame_i * sprite.height;

    var uniforms = {
        'sheetSize': [base_tex.width, base_tex.height],
        'sliceFrac': [slice_frac],
        'pos': [sprite.ref_x, sprite.ref_y, sprite.ref_z],
        'base': [offset_x + (sprite.flip ? sprite.width : 0),
                 offset_y],
        'size': [(sprite.flip ? -sprite.width : sprite.width),
                 sprite.height],
        'anchor': [sprite.anchor_x, sprite.anchor_y],
        'color': [this.body_color[0] / 255.0,
                  this.body_color[1] / 255.0,
                  this.body_color[2] / 255.0,
                  1.0],
    };

    var obj = r.classes.pony._obj;
    obj.draw(fb_idx, 0, 6, uniforms, {}, textures);
};

PonyAppearance.prototype.draw2D = function(ctx, view_base, sprite) {
    var x = sprite.ref_x - sprite.anchor_x - view_base[0];
    var y = sprite.ref_y - sprite.ref_z - sprite.anchor_y - view_base[1];
    var w = sprite.width;
    var h = sprite.height;

    var buf = new OffscreenContext(w, h);
    var buf_x = 0;
    var buf_y = 0;

    if (sprite.flip) {
        buf.scale(-1, 1);
        buf_x = -buf_x - w;
    }

    var off_x = sprite.frame_j * width;
    var off_y = sprite.frame_i * height;

    buf.globalCompositeOperation = 'copy';
    buf.drawImage(this.base_img,
            off_x, off_y, w, h,
            buf_x, buf_y, w, h);

    buf.globalCompositeOperation = 'source-in';
    buf.fillStyle = 'rgb(' + this.body_color.join(',') + ')';
    buf.fillRect(buf_x, buf_y, w, h);

    buf.globalCompositeOperation = 'multiply';
    buf.drawImage(this.base_img,
            off_x, off_y, w, h,
            buf_x, buf_y, w, h);

    ctx.drawImage(buf.canvas, x, y);
};


/** @constructor */
function PonyAppearanceClass(gl, assets) {
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
    var attributes = {
        'posOffset': attribute(buffer, 2, gl.UNSIGNED_BYTE, false, 0, 0),
    };
    var textures = {};
    for (var i = 0; i < 8; ++i) {
        textures['sheetSampler[' + i + ']'] = null;
    }

    this._obj = new GlObject(gl, programs, uniforms, attributes, textures);
}
exports.PonyAppearanceClass = PonyAppearanceClass;
console.log('gone', exports.PonyAppearanceClass);

PonyAppearanceClass.prototype.setCamera = function(pos, size) {
    this._obj.setUniformValue('cameraPos', pos);
    this._obj.setUniformValue('cameraSize', size);
};
