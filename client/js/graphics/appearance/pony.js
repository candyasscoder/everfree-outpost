var Config = require('config').Config;
var Sprite = require('graphics/sprite').Sprite;
var OffscreenContext = require('graphics/canvas').OffscreenContext;
var glutil = require('graphics/glutil');
var named = require('graphics/draw/named');


var TRIBE_NAME = ['E', 'P', 'U', 'A'];
var COLOR_RAMP = [0x44, 0x88, 0xcc, 0xff];

/** @constructor */
function PonyAppearance(assets, bits, name) {
    var tribe = (bits >> 6) & 3;
    // TODO: use a SpriteSheet object that contains all the sheet images
    this.base_img = assets['pony_f_base_' + TRIBE_NAME[tribe] + '-0'];
    this.eyes_img = assets['pony_f_eyes_0-0'];
    this.mane_img = assets['pony_f_mane_0-0'];
    this.tail_img = assets['pony_f_tail_0-0'];

    var hat = ((bits >> 8) & 1) != 0;
    this.equip_img = [
        hat ? assets['pony_f_equip0_0-0'] : null,
        null,
        null,
    ];
    this.has_equip = [
        hat,
        false,
        false,
    ];

    var r = (bits >> 4) & 3;
    var g = (bits >> 2) & 3;
    var b = (bits >> 0) & 3;
    this.hair_color = [
        COLOR_RAMP[r] / 255.0,
        COLOR_RAMP[g] / 255.0,
        COLOR_RAMP[b] / 255.0,
    ];
    this.body_color = [
        COLOR_RAMP[r + 1] / 255.0,
        COLOR_RAMP[g + 1] / 255.0,
        COLOR_RAMP[b + 1] / 255.0,
    ];

    this.name = name;
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
        'sheetBase': base_tex,
        'sheetEyes': r.cacheTexture(this.eyes_img),
        'sheetMane': r.cacheTexture(this.mane_img),
        'sheetTail': r.cacheTexture(this.tail_img),
    };

    for (var i = 0; i < 3; ++i) {
        if (this.equip_img[i] != null) {
            textures['sheetEquip[' + i + ']'] = r.cacheTexture(this.equip_img[i]);
        }
    }

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

        'colorBody': this.body_color,
        'colorHair': this.hair_color,
        'hasEquip': this.has_equip,
    };

    var cls = r.classes.pony;
    var obj = cls._obj;
    obj.draw(fb_idx, 0, 6, uniforms, {}, textures);


    if (Config.render_names.get()) {
        var off = cls.getNameOffset(this.name);

        var uniforms = {
            'sliceFrac': [slice_frac],
            // TODO: hardcoded name positioning, should be computed somehow to
            // center the name at a reasonable height.
            'pos': [sprite.ref_x, sprite.ref_y, sprite.ref_z + 90 - 22],
            'base': [off.x, off.y],
            'size': [named.NAME_WIDTH, named.NAME_HEIGHT],
            'anchor': [named.NAME_WIDTH / 2, named.NAME_HEIGHT],
        };
        cls._name_obj.draw(fb_idx, 0, 6, uniforms, {}, {});
    }
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

    var off_x = sprite.frame_j * w;
    var off_y = sprite.frame_i * h;

    // TODO: fix alpha
    function draw_layer(img) {
        buf.globalCompositeOperation = 'copy';
        buf.drawImage(img,
                off_x, off_y, w, h,
                buf_x, buf_y, w, h);
        var img = buf.getImageData(0, 0, w, h);

        for (var i = 3; i < img.data.length; i += 4) {
            if (img.data[i] != 0) {
                img.data[i] = 255;
            }
        }
        buf.putImageData(img, 0, 0);

        ctx.drawImage(buf.canvas, x, y);
    }

    function draw_layer_tinted(img, color) {
        buf.globalCompositeOperation = 'copy';
        buf.drawImage(img,
                off_x, off_y, w, h,
                buf_x, buf_y, w, h);
        var orig = buf.getImageData(0, 0, w, h);

        buf.globalCompositeOperation = 'multiply';
        buf.fillStyle = 'rgb(' + [color[0] * 255, color[1] * 255, color[2] * 255].join(',') + ')';
        buf.fillRect(buf_x, buf_y, w, h);
        var img = buf.getImageData(0, 0, w, h);

        for (var i = 3; i < img.data.length; i += 4) {
            if (orig.data[i] == 0) {
                img.data[i] = 0;
            } else {
                img.data[i] = 255;
            }
        }
        buf.putImageData(img, 0, 0);

        ctx.drawImage(buf.canvas, x, y);
    }

    draw_layer_tinted(this.base_img, this.body_color);
    draw_layer(this.eyes_img);
    draw_layer_tinted(this.mane_img, this.hair_color);
    draw_layer_tinted(this.tail_img, this.hair_color);
};


/** @constructor */
function PonyAppearanceClass(gl, assets) {
    this._obj = make_pony_obj(gl, assets);

    this._name_tex = new glutil.Texture(gl);
    this._name_obj = make_name_obj(gl, assets, this._name_tex);

    this._name_buf = new named.NameBuffer(assets);
}
exports.PonyAppearanceClass = PonyAppearanceClass;

PonyAppearanceClass.prototype.setCamera = function(pos, size) {
    this._obj.setUniformValue('cameraPos', pos);
    this._obj.setUniformValue('cameraSize', size);

    this._name_obj.setUniformValue('cameraPos', pos);
    this._name_obj.setUniformValue('cameraSize', size);
};

PonyAppearanceClass.prototype.getNameOffset = function(name) {
    var off = this._name_buf.offset(name);
    if (off.created) {
        this._name_tex.loadImage(this._name_buf.image());
    }
    return off;
};

function make_pony_obj(gl, assets) {
    var vert = assets['sprite.vert'];
    var frag = assets['app_pony.frag'];
    var programs = glutil.buildPrograms(gl, vert, frag, 2);

    var buffer = new glutil.Buffer(gl);
    buffer.loadData(new Uint8Array([
            0, 0,
            0, 1,
            1, 1,

            0, 0,
            1, 1,
            1, 0,
    ]));

    var uniform = glutil.uniform;
    var attribute = glutil.attribute;

    var uniforms = {
        'cameraPos': uniform('vec2', null),
        'cameraSize': uniform('vec2', null),
        'sheetSize': uniform('vec2', null),
        'sliceFrac': uniform('float', null),
        'pos': uniform('vec3', null),
        'base': uniform('vec2', null),
        'size': uniform('vec2', null),
        'anchor': uniform('vec2', null),

        'colorBody': uniform('vec3', null),
        'colorHair': uniform('vec3', null),
        'hasEquip': uniform('bool', null),
    };
    var attributes = {
        'posOffset': attribute(buffer, 2, gl.UNSIGNED_BYTE, false, 0, 0),
    };
    var textures = {
        'sheetBase': null,
        'sheetMane': null,
        'sheetTail': null,
        'sheetEyes': null,
        'sheetEquip[0]': null,
        'sheetEquip[1]': null,
        'sheetEquip[2]': null,
    };

    return new glutil.GlObject(gl, programs, uniforms, attributes, textures);
}

function make_name_obj(gl, assets, name_tex) {
    var vert = assets['sprite.vert'];
    var frag = assets['sprite.frag'];
    var programs = glutil.buildPrograms(gl, vert, frag, 2);

    var buffer = new glutil.Buffer(gl);
    buffer.loadData(new Uint8Array([
            0, 0,
            0, 1,
            1, 1,

            0, 0,
            1, 1,
            1, 0,
    ]));

    var uniform = glutil.uniform;
    var attribute = glutil.attribute;

    var uniforms = {
        'cameraPos': uniform('vec2', null),
        'cameraSize': uniform('vec2', null),
        'sheetSize': uniform('vec2', [named.NAME_BUFFER_WIDTH, named.NAME_BUFFER_HEIGHT]),
        'sliceFrac': uniform('float', null),
        'pos': uniform('vec3', null),
        'base': uniform('vec2', null),
        'size': uniform('vec2', null),
        'anchor': uniform('vec2', null),
    };
    var attributes = {
        'posOffset': attribute(buffer, 2, gl.UNSIGNED_BYTE, false, 0, 0),
    };
    var textures = {
        'sheetSampler': name_tex,
    };

    return new glutil.GlObject(gl, programs, uniforms, attributes, textures);
}
