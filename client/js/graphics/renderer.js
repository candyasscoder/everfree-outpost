var Asm = require('asmlibs').Asm;
var getRendererHeapSize = require('asmlibs').getRendererHeapSize;
var getGraphics2HeapSize = require('asmlibs').getGraphics2HeapSize;
var OffscreenContext = require('graphics/canvas').OffscreenContext;
var TileDef = require('data/chunk').TileDef;
var CHUNK_SIZE = require('data/chunk').CHUNK_SIZE;
var TILE_SIZE = require('data/chunk').TILE_SIZE;
var LOCAL_SIZE = require('data/chunk').LOCAL_SIZE;
var Program = require('graphics/glutil').Program;
var Texture = require('graphics/glutil').Texture;
var Buffer = require('graphics/glutil').Buffer;
var Framebuffer = require('graphics/glutil').Framebuffer;

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
    this._asm2 = new Asm(getGraphics2HeapSize());

    this.texture_cache = new WeakMap();
}
exports.Renderer = Renderer;


// Renderer initialization

Renderer.prototype.initGl = function(assets) {
    var gl = this.gl;

    gl.clearColor(0, 0, 0, 1);
    gl.enable(gl.BLEND);
    gl.blendFunc(gl.ONE, gl.ONE_MINUS_SRC_ALPHA);

    var atlas = assets['tiles'];
    var atlas_tex = this.cacheTexture(atlas);

    this.blit = build_blit(gl, assets);
    this.terrain_block = build_terrain_block(gl, assets, atlas_tex);

    var chunk_px = CHUNK_SIZE * TILE_SIZE;
    this.chunk_fbs = new Array(LOCAL_SIZE * LOCAL_SIZE);
    for (var i = 0; i < LOCAL_SIZE * LOCAL_SIZE; ++i) {
        this.chunk_fbs[i] = new Framebuffer(gl, chunk_px, chunk_px);
    }
};

function build_terrain_block(gl, assets, atlas_tex) {
    var vert = assets['terrain_block.vert'];
    var frag = assets['terrain_block.frag'];
    var program = new Program(gl, vert, frag);

    var uniforms = {
        'atlasSize': uniform('vec2', [(atlas_tex.width / TILE_SIZE)|0,
                                      (atlas_tex.height / TILE_SIZE)|0]),
    };

    var attributes = {
        'position': attribute(null, 3, gl.UNSIGNED_BYTE, false, 8, 0),
        'texCoord': attribute(null, 2, gl.UNSIGNED_BYTE, false, 8, 4),
    };

    var textures = {
        'atlasTex': atlas_tex,
    };

    return new GlObject(gl, program, uniforms, attributes, textures);
}

function build_blit(gl, assets) {
    var vert = assets['blit.vert'];
    var frag = assets['blit.frag'];
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
        'rectPos': uniform('vec2', null),
        'rectSize': uniform('vec2', [CHUNK_SIZE * TILE_SIZE, CHUNK_SIZE * TILE_SIZE]),
        'cameraPos': uniform('vec2', null),
        'cameraSize': uniform('vec2', null),
    };

    var attributes = {
        'posOffset': attribute(buffer, 2, gl.UNSIGNED_BYTE, false, 0, 0),
    };

    var textures = {
        'tex': null,
    };

    return new GlObject(gl, program, uniforms, attributes, textures);
}


// Texture object management

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


// Data loading

Renderer.prototype.loadBlockData = function(blocks) {
    var view2 = this._asm2.blockDataView2();
    for (var i = 0; i < blocks.length; ++i) {
        var block = blocks[i];
        var base = i * 4;
        view2[base + 0] = block.front;
        view2[base + 1] = block.back;
        view2[base + 2] = block.top;
        view2[base + 3] = block.bottom;
    }
};

Renderer.prototype.loadChunk = function(i, j, chunk) {
    var idx = i * LOCAL_SIZE + j;

    this._asm2.chunkView2().set(chunk._tiles);
    this._asm2.loadChunk2(j, i);

    this._refreshTerrain(i, j);
    this._refreshTerrain(i - 1, j);
};

Renderer.prototype._refreshTerrain = function(i, j) {
    i = i & (LOCAL_SIZE - 1);
    j = j & (LOCAL_SIZE - 1);
    var idx = i * LOCAL_SIZE + j;

    var geom = this._asm2.generateGeometry2(j, i);

    var gl = this.gl;
    var fb = this.chunk_fbs[idx];
    fb.bind();
    gl.viewport(0, 0, fb.width, fb.height);
    gl.clearDepth(0.0);
    gl.clearColor(0, 0, 0, 0);
    gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
    gl.enable(gl.DEPTH_TEST);
    gl.depthFunc(gl.GEQUAL);

    var buffer = new Buffer(gl);
    buffer.loadData(geom);

    this.terrain_block.draw(0, geom.length / 8, {}, {
        'position': buffer,
        'texCoord': buffer,
    }, {});

    fb.unbind();
    gl.disable(gl.DEPTH_TEST);
};


// Render

Renderer.prototype.render = function(ctx, sx, sy, sw, sh, sprites, mask_info) {
    var gl = this.gl;

    this.blit.setUniformValue('cameraPos', [sx, sy]);
    this.blit.setUniformValue('cameraSize', [sw, sh]);

    var chunk_px = CHUNK_SIZE * TILE_SIZE;
    var cx0 = ((sx|0) / chunk_px)|0;
    var cx1 = (((sx|0) + (sw|0) + chunk_px) / chunk_px)|0;
    var cy0 = ((sy|0) / chunk_px)|0;
    var cy1 = (((sy|0) + (sh|0) + chunk_px) / chunk_px)|0;

    for (var cy = cy0; cy < cy1; ++cy) {
        for (var cx = cx0; cx < cx1; ++cx) {
            var idx = ((cy & (LOCAL_SIZE - 1)) * LOCAL_SIZE) + (cx & (LOCAL_SIZE - 1));
            this.blit.draw(0, 6, {
                'rectPos': [cx * CHUNK_SIZE * TILE_SIZE,
                            cy * CHUNK_SIZE * TILE_SIZE],
            }, {}, {
                'tex': this.chunk_fbs[idx].texture,
            });
        }
    }
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
