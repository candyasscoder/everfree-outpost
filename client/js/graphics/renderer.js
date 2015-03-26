var Asm = require('asmlibs').Asm;
var getRendererHeapSize = require('asmlibs').getRendererHeapSize;
var OffscreenContext = require('graphics/canvas').OffscreenContext;
var TileDef = require('data/chunk').TileDef;
var CHUNK_SIZE = require('data/chunk').CHUNK_SIZE;
var TILE_SIZE = require('data/chunk').TILE_SIZE;
var LOCAL_SIZE = require('data/chunk').LOCAL_SIZE;
var Program = require('graphics/glutil').Program;
var Texture = require('graphics/glutil').Texture;
var Buffer = require('graphics/glutil').Buffer;

var GlObject = require('graphics/glutil').GlObject;
var uniform = require('graphics/glutil').uniform;
var attribute = require('graphics/glutil').attribute;

var Simple3D = require('graphics/draw/simple').Simple3D;
var Layered3D = require('graphics/draw/layered').Layered3D;
var Named3D = require('graphics/draw/named').Named3D;

var Vec = require('util/vec').Vec;


/** @constructor */
function Renderer(gl) {
    this.gl = gl;
    this._asm = new Asm(getRendererHeapSize());

    this._chunk_buffer = new Array(LOCAL_SIZE * LOCAL_SIZE);
    this._chunk_points = new Array(LOCAL_SIZE * LOCAL_SIZE);
    for (var i = 0; i < LOCAL_SIZE * LOCAL_SIZE; ++i) {
        this._chunk_buffer[i] = new Buffer(gl);
        this._chunk_points[i] = 0;
    }
}
exports.Renderer = Renderer;

Renderer.prototype.initGl = function(assets) {
    var gl = this.gl;

    gl.clearColor(0, 0, 0, 1);
    gl.enable(gl.BLEND);
    gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA);


    var terrain_vert = assets['terrain.vert'];
    var terrain_frag = assets['terrain.frag'];
    var terrain_program = new Program(gl, terrain_vert, terrain_frag);

    var atlas = assets['tiles'];
    var atlas_texture = new Texture(gl);
    atlas_texture.loadImage(atlas);

    var terrain_uniforms = {
        'atlasSize': uniform('vec2',
                [(atlas.width / TILE_SIZE)|0,
                 (atlas.height / TILE_SIZE)|0]),
        'cameraPos': uniform('vec2', null),
        'cameraSize': uniform('vec2', null),
        'chunkPos': uniform('vec2', null),
        'maskCenter': uniform('vec2', null),
        'maskRadius2': uniform('float', null),
    };

    var terrain_attributes = {
        'position': attribute(null, 3, gl.UNSIGNED_BYTE, false, 8, 0),
        'texCoord': attribute(null, 2, gl.UNSIGNED_BYTE, false, 8, 4),
    };

    this.terrain_obj = new GlObject(gl, terrain_program,
            terrain_uniforms,
            terrain_attributes,
            {'atlasSampler': atlas_texture});


    this.sprite_classes = {
        'simple': new Simple3D(gl, assets),
        'layered': new Layered3D(gl, assets),
        'named': new Named3D(gl, assets),
    };

    this.texture_cache = new WeakMap();
};

Renderer.prototype.cacheTexture = function(image) {
    var tex = this.texture_cache.get(image);
    if (tex != null) {
        // Cache hit
        return tex;
    }

    // Cache miss - create a new texture
    var tex = new Texture(this.gl);
    tex.loadImage(image);
    this.texture_cache.set(image, tex);
    return tex;
};

Renderer.prototype.refreshTexture = function(image) {
    var tex = this.texture_cache.get(image);
    if (tex != null) {
        tex.loadImage(image);
    }
};

Renderer.prototype.loadBlockData = function(blocks) {
    var view = this._asm.blockDataView();
    for (var i = 0; i < blocks.length; ++i) {
        var block = blocks[i];
        var base = i * 4;
        view[base + 0] = block.front;
        view[base + 1] = block.back;
        view[base + 2] = block.top;
        view[base + 3] = block.bottom;
    }
};

Renderer.prototype.loadChunk = function(i, j, chunk) {
    var idx = i * LOCAL_SIZE + j;

    this._asm.chunkDataView().set(chunk._tiles);
    i = (idx / LOCAL_SIZE)|0;
    j = (idx % LOCAL_SIZE);
    this._asm.updateXvData(i, j);

    this._refreshGeometry(i, j);
    this._refreshGeometry(i - 1, j);
};

Renderer.prototype._refreshGeometry = function(i, j) {
    i = i & (LOCAL_SIZE - 1);
    j = j & (LOCAL_SIZE - 1);
    var idx = i * LOCAL_SIZE + j;

    var geom = this._asm.generateGeometry(i, j);
    this._chunk_buffer[idx].loadData(geom);
    this._chunk_points[idx] = (geom.length / 4)|0;
};

Renderer.prototype.render = function(ctx, sx, sy, sw, sh, sprites, mask_info) {
    var gl = this.gl;

    this.terrain_obj.setUniformValue('cameraPos', [sx, sy]);
    this.terrain_obj.setUniformValue('cameraSize', [sw, sh]);
    this.terrain_obj.setUniformValue('maskCenter', mask_info.center);
    this.terrain_obj.setUniformValue('maskRadius2', [mask_info.radius * mask_info.radius]);

    for (var k in this.sprite_classes) {
        var cls = this.sprite_classes[k];
        cls.setCamera(sx, sy, sw, sh);
    }

    var this_ = this;

    var cur_cx = -1;
    var cur_cy = -1;
    var cur_indices = [];

    function buffer_terrain(cx, cy, begin, end) {
        if (cx != cur_cx || cy != cur_cy) {
            flush_terrain();
            cur_cx = cx;
            cur_cy = cy;
        }
        cur_indices.push([6 * begin, 6 * (end - begin)]);
    }

    function flush_terrain() {
        if (cur_indices.length == 0) {
            return;
        }

        var i = cur_cy % LOCAL_SIZE;
        var j = cur_cx % LOCAL_SIZE;
        var idx = i * LOCAL_SIZE + j;
        var buffer = this_._chunk_buffer[idx];

        this_.terrain_obj.drawMulti(cur_indices,
                {'chunkPos': [cur_cx, cur_cy]},
                {'position': buffer,
                 'texCoord': buffer},
                {});

        cur_indices.length = 0;
    }

    function draw_sprite(id, x, y, w, h) {
        flush_terrain();

        var sprite = sprites[id];
        // Coordinates where the sprite would normally be displayed.
        var x0 = sprite.ref_x - sprite.anchor_x;
        var y0 = sprite.ref_y - sprite.ref_z - sprite.anchor_y;
        // The region x,y,w,h is x0,y0,sprite.width,sprite.height clipped to
        // lie within some other region.
        var clip_x = x - x0;
        var clip_y = y - y0;

        var cls = this_.sprite_classes[sprite.cls];
        console.assert(cls != null,
                'unknown sprite class', sprite.cls);

        cls.draw(this_, sprite,
                 x0, y0,
                 clip_x, clip_y, w, h);
    }

    this._asm.render(sx, sy, sw, sh, sprites, buffer_terrain, draw_sprite);

    flush_terrain();
};


/** @constructor */
function Sprite(width, height, anchor_x, anchor_y, cls, extra) {
    this.width = width;
    this.height = height;

    this.ref_x = 0;
    this.ref_y = 0;
    this.ref_z = 0;
    this.anchor_x = anchor_x;
    this.anchor_y = anchor_y;

    this.flip = false;

    this.cls = cls;
    this.extra = extra;
}
exports.Sprite = Sprite;

Sprite.prototype.refPosition = function() {
    return new Vec(this.ref_x, this.ref_y, this.ref_z);
};

Sprite.prototype.setClass = function(cls, extra) {
    this.cls = cls;
    this.extra = extra;
};

Sprite.prototype.setFlip = function(flip) {
    this.flip = flip;
};

Sprite.prototype.setPos = function(ref_pos) {
    this.ref_x = ref_pos.x;
    this.ref_y = ref_pos.y;
    this.ref_z = ref_pos.z;
};


/** @constructor */
function SpriteBase(width, height, anchor_x, anchor_y, extra) {
    this.width = width;
    this.height = height;
    this.anchor_x = anchor_x;
    this.anchor_y = anchor_y;
    this.cls = extra.getClass();
    this.extra = extra;
}
exports.SpriteBase = SpriteBase;

SpriteBase.prototype.instantiate = function() {
    return new Sprite(this.width,
                      this.height,
                      this.anchor_x,
                      this.anchor_y,
                      this.cls,
                      this.extra);
};
