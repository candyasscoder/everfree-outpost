var Asm = require('asmlibs').Asm;
var getRendererHeapSize = require('asmlibs').getRendererHeapSize;
var getGraphicsHeapSize = require('asmlibs').getGraphicsHeapSize;
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
    this._asm = new Asm(getGraphicsHeapSize());
    this._asm.initStructureBuffer();

    this.texture_cache = new WeakMap();
    this.terrain_cache = new RenderCache(gl);
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

    var struct_sheet = assets['structures0'];
    var struct_sheet_tex = this.cacheTexture(struct_sheet);

    var struct_depth = assets['structdepth0'];
    var struct_depth_tex = this.cacheTexture(struct_depth);

    this.blit = build_blit(gl, assets);
    this.terrain_block = build_terrain_block(gl, assets, atlas_tex);
    this.structure = build_structure(gl, assets, struct_sheet_tex, struct_depth_tex);

    this.sprite_classes = {
        'simple': new Simple3D(gl, assets),
        'layered': new Layered3D(gl, assets),
        'named': new Named3D(gl, assets),
    };
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
        'imageTex': null,
        'depthTex': null,
    };

    return new GlObject(gl, program, uniforms, attributes, textures);
}

function build_structure(gl, assets, sheet_tex, depth_tex) {
    var vert = assets['structure.vert'];
    var frag = assets['structure.frag'];
    var program = new Program(gl, vert, frag);

    var uniforms = {
        'sheetSize': uniform('vec2', [sheet_tex.width, sheet_tex.height]),
    };

    var attributes = {
        'position': attribute(null, 3, gl.SHORT, false, 16, 0),
        'texCoord': attribute(null, 2, gl.UNSIGNED_SHORT, false, 16, 8),
    };

    var textures = {
        'sheetTex': sheet_tex,
        'depthTex': depth_tex,
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
    this._asm.chunkView().set(chunk._tiles);
    this._asm.loadChunk(j, i);

    this.terrain_cache.invalidate(i * LOCAL_SIZE + j);
    this.terrain_cache.invalidate((i - 1) * LOCAL_SIZE + j);
};

Renderer.prototype.loadTemplateData = function(templates) {
    var view8 = this._asm.templateDataView8();
    var view16 = this._asm.templateDataView16();
    for (var i = 0; i < templates.length; ++i) {
        var template = templates[i];
        var out8 = view8.subarray(i * 12, (i + 1) * 12);
        var out16 = view16.subarray(i * 6, (i + 1) * 6);

        out8[0] = template.size.x;
        out8[1] = template.size.y;
        out8[2] = template.size.z;
        out8[3] = template.sheet;
        out16[2] = template.display_size[0];
        out16[3] = template.display_size[1];
        out16[4] = template.display_offset[0];
        out16[5] = template.display_offset[1];
    }
};

Renderer.prototype.addStructure = function(x, y, z, template_id) {
    return this._asm.addStructure(x, y, z, template_id);
};

Renderer.prototype.removeStructure = function(idx) {
    this._asm.removeStructure(idx);
};


// Render

Renderer.prototype._renderTerrain = function(fb, cx, cy) {
    var geom = this._asm.generateGeometry(cx, cy);

    var gl = this.gl;
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

Renderer.prototype._renderStructures = function(fb, cx, cy) {
    var gl = this.gl;
    fb.bind();
    gl.viewport(0, 0, fb.width, fb.height);
    gl.clearDepth(0.0);
    gl.clearColor(0, 0, 0, 0);
    gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
    gl.enable(gl.DEPTH_TEST);
    gl.depthFunc(gl.GEQUAL);

    this._asm.resetStructureGeometry();
    var more = true;
    while (more) {
        var result = this._asm.generateStructureGeometry(cx, cy);
        var geom = result.geometry;
        more = result.more;
        // TODO: use result.sheet

        var buffer = new Buffer(gl);
        buffer.loadData(geom);

        this.structure.draw(0, geom.length / 8, {}, {
            'position': buffer,
            'texCoord': buffer,
        }, {});
    }

    fb.unbind();
    gl.disable(gl.DEPTH_TEST);
};

Renderer.prototype.render = function(ctx, sx, sy, sw, sh, sprites, mask_info) {
    var gl = this.gl;

    this.blit.setUniformValue('cameraPos', [sx, sy]);
    this.blit.setUniformValue('cameraSize', [sw, sh]);

    for (var k in this.sprite_classes) {
        var cls = this.sprite_classes[k];
        cls.setCamera(sx, sy, sw, sh);
    }

    var chunk_px = CHUNK_SIZE * TILE_SIZE;
    var cx0 = ((sx|0) / chunk_px)|0;
    var cx1 = (((sx|0) + (sw|0) + chunk_px) / chunk_px)|0;
    var cy0 = ((sy|0) / chunk_px)|0;
    var cy1 = (((sy|0) + (sh|0) + chunk_px) / chunk_px)|0;

    var chunk_idxs = new Array((cx1 - cx0) * (cy1 - cy0));

    var i = 0;
    for (var cy = cy0; cy < cy1; ++cy) {
        for (var cx = cx0; cx < cx1; ++cx) {
            var idx = ((cy & (LOCAL_SIZE - 1)) * LOCAL_SIZE) + (cx & (LOCAL_SIZE - 1));
            chunk_idxs[i] = idx;
            ++i;
        }
    }

    var this_ = this;
    this.terrain_cache.populate(chunk_idxs, function(idx, fbs) {
        var cx = (idx % LOCAL_SIZE)|0;
        var cy = (idx / LOCAL_SIZE)|0;
        this_._renderTerrain(fbs.terrain, cx, cy);
        this_._renderStructures(fbs.structures, cx, cy);
    });

    // `populate` may call `_renderTerrain`, which changes the OpenGL viewport.
    gl.viewport(0, 0, gl.canvas.width, gl.canvas.height);

    gl.enable(gl.DEPTH_TEST);
    gl.depthFunc(gl.GEQUAL);

    for (var cy = cy0; cy < cy1; ++cy) {
        for (var cx = cx0; cx < cx1; ++cx) {
            var idx = ((cy & (LOCAL_SIZE - 1)) * LOCAL_SIZE) + (cx & (LOCAL_SIZE - 1));

            this.blit.draw(0, 6, {
                'rectPos': [cx * CHUNK_SIZE * TILE_SIZE,
                            cy * CHUNK_SIZE * TILE_SIZE],
            }, {}, {
                'imageTex': this.terrain_cache.get(idx).terrain.texture,
                'depthTex': this.terrain_cache.get(idx).terrain.depth_texture,
            });

            this.blit.draw(0, 6, {
                'rectPos': [cx * CHUNK_SIZE * TILE_SIZE,
                            cy * CHUNK_SIZE * TILE_SIZE],
            }, {}, {
                'imageTex': this.terrain_cache.get(idx).structures.texture,
                'depthTex': this.terrain_cache.get(idx).structures.depth_texture,
            });
        }
    }

    for (var i = 0; i < sprites.length; ++i) {
        var sprite = sprites[i];
        var cls = this.sprite_classes[sprite.cls];
        cls.draw(this, sprite);
    }

    gl.disable(gl.DEPTH_TEST);
};



/** @constructor */
function RenderCache(gl) {
    this.gl = gl;
    this.slots = [];
    this.users = [];

    this.map = new Array(LOCAL_SIZE * LOCAL_SIZE);
    for (var i = 0; i < this.map.length; ++i) {
        this.map[i] = -1;
    }

    // `users` maps slots to indexes.  `map` maps indexes to slots.  `map` is
    // not always kept up to date, so it's necessary to check that
    // `users[slot] == idx` before relying on the result of a `map` lookup.
}

RenderCache.prototype._addSlot = function() {
    var chunk_px = CHUNK_SIZE * TILE_SIZE;
    this.slots.push({
        terrain: new Framebuffer(this.gl, chunk_px, chunk_px),
        structures: new Framebuffer(this.gl, chunk_px, chunk_px),
    });
    this.users.push(-1);
};

RenderCache.prototype.populate = function(idxs, callback) {
    // First, collect any slot/idx pairs that can be reused.  Clear all
    // remaining slots (set `user[slot]` to -1).
    var new_users = new Array(this.users.length);
    for (var i = 0; i < new_users.length; ++i) {
        new_users[i] = -1;
    }

    for (var i = 0; i < idxs.length; ++i) {
        var idx = idxs[i];
        var slot = this.map[idx];
        if (slot != -1 && this.users[slot] == idx) {
            new_users[slot] = idx;
        }
    }

    this.users = new_users;

    // Now make a second pass to find slots for all remaining `idxs`.
    var free = 0;
    for (var i = 0; i < idxs.length; ++i) {
        var idx = idxs[i];
        var slot = this.map[idx];
        if (slot == -1 || this.users[slot] != idx) {
            // Find or create a free slot
            while (free < this.users.length && this.users[free] != -1) {
                ++free;
            }
            if (free == this.users.length) {
                this._addSlot();
            }

            // Populate the slot and assign it to `idx`.
            callback(idx, this.slots[free]);
            this.map[idx] = free;
            this.users[free] = idx;
        }
    }
};

RenderCache.prototype.get = function(idx) {
    var slot = this.map[idx];
    if (slot == -1 || this.users[slot] != idx) {
        return null;
    } else {
        return this.slots[slot];
    }
};

RenderCache.prototype.invalidate = function(idx) {
    var slot = this.map[idx];
    if (slot != -1 && this.users[slot] == idx) {
        this.users[slot] = -1;
    }
    this.map[idx] = -1;
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
