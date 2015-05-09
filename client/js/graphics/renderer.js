var Asm = require('asmlibs').Asm;
var getRendererHeapSize = require('asmlibs').getRendererHeapSize;
var getGraphicsHeapSize = require('asmlibs').getGraphicsHeapSize;
var OffscreenContext = require('graphics/canvas').OffscreenContext;
var BlockDef = require('data/chunk').BlockDef;
var TemplateDef = require('data/templates').TemplateDef;
var CHUNK_SIZE = require('data/chunk').CHUNK_SIZE;
var TILE_SIZE = require('data/chunk').TILE_SIZE;
var LOCAL_SIZE = require('data/chunk').LOCAL_SIZE;
var buildPrograms = require('graphics/glutil').buildPrograms;
var Texture = require('graphics/glutil').Texture;
var Buffer = require('graphics/glutil').Buffer;
var Framebuffer = require('graphics/glutil').Framebuffer;

var GlObject = require('graphics/glutil').GlObject;
var uniform = require('graphics/glutil').uniform;
var attribute = require('graphics/glutil').attribute;

var Simple3D = require('graphics/draw/simple').Simple3D;
var Layered3D = require('graphics/draw/layered').Layered3D;
var Named3D = require('graphics/draw/named').Named3D;
var PonyOutline3D = require('graphics/draw/ponyoutline').PonyOutline3D;

var Vec = require('util/vec').Vec;

var CHUNK_PX = CHUNK_SIZE * TILE_SIZE;

/** @constructor */
function Renderer(gl) {
    this.gl = gl;
    this._asm = new Asm(getGraphicsHeapSize());
    this._asm.initStructureBuffer();
    this._asm.initLightState();

    this.texture_cache = new WeakMap();
    this.terrain_cache = new RenderCache(gl, function(gl) {
        return {
            image: new Framebuffer(gl, CHUNK_PX, CHUNK_PX, 2),
        };
    });
    this.sliced_cache = new RenderCache(gl, function(gl) {
        return {
            image: new Framebuffer(gl, CHUNK_PX, CHUNK_PX, 2),
        };
    });
    this.last_slice_z = -1;
}
exports.Renderer = Renderer;


// Renderer initialization

Renderer.prototype.initGl = function(assets) {
    var gl = this.gl;

    var atlas = assets['tiles'];
    var atlas_tex = this.cacheTexture(atlas);

    var struct_sheet = assets['structures0'];
    var struct_sheet_tex = this.cacheTexture(struct_sheet);

    var struct_depth = assets['structdepth0'];
    var struct_depth_tex = this.cacheTexture(struct_depth);

    var blits = build_blits(gl, assets);
    this.blit = blits.normal;
    this.blit_sliced = blits.sliced;
    this.output = blits.output;
    this.post_filter = blits.post;
    this.terrain_block = build_terrain_block(gl, assets, atlas_tex);
    this.structure = build_structure(gl, assets, struct_sheet_tex, struct_depth_tex);
    this.light = build_light(gl, assets);

    this.sprite_classes = {
        'simple': new Simple3D(gl, assets),
        'layered': new Layered3D(gl, assets),
        'named': new Named3D(gl, assets),
        'pony_outline': new PonyOutline3D(gl, assets),
    };

    this.last_sw = -1;
    this.last_sh = -1;
    this.fbs = [null, null];
};

function build_terrain_block(gl, assets, atlas_tex) {
    var vert = assets['terrain_block.vert'];
    var frag = assets['terrain_block.frag'];
    var programs = buildPrograms(gl, vert, frag, 2);

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

    return new GlObject(gl, programs, uniforms, attributes, textures);
}

function build_blits(gl, assets) {
    var vert = assets['blit.vert'];
    var vert_fullscreen = assets['blit_fullscreen.vert'];

    var frag = assets['blit.frag'];
    var programs = buildPrograms(gl, vert, frag, 2);

    var frag_sliced = assets['blit_sliced.frag'];
    var programs_sliced = buildPrograms(gl, vert, frag_sliced, 2);

    var frag_output = assets['blit_output.frag'];
    var programs_output = buildPrograms(gl, vert_fullscreen, frag_output, 1);

    var frag_post = assets['blit_post.frag'];
    var programs_post = buildPrograms(gl, vert_fullscreen, frag_post, 1);

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
        'image0Tex': null,
        'image1Tex': null,
        'depthTex': null,
    };

    var normal = new GlObject(gl, programs, uniforms, attributes, textures);


    var uniforms = {
        'rectPos': uniform('vec2', null),
        'rectSize': uniform('vec2', [CHUNK_SIZE * TILE_SIZE, CHUNK_SIZE * TILE_SIZE]),
        'cameraPos': uniform('vec2', null),
        'cameraSize': uniform('vec2', null),
        'sliceFrac': uniform('float', null),
    };

    var attributes = {
        'posOffset': attribute(buffer, 2, gl.UNSIGNED_BYTE, false, 0, 0),
    };

    var textures = {
        'upperImage0Tex': null,
        'upperImage1Tex': null,
        'upperDepthTex': null,
        'lowerImage0Tex': null,
        'lowerImage1Tex': null,
        'lowerDepthTex': null,
    };

    var sliced = new GlObject(gl, programs_sliced, uniforms, attributes, textures);


    var uniforms = {};

    var attributes = {
        'posOffset': attribute(buffer, 2, gl.UNSIGNED_BYTE, false, 0, 0),
    };

    var textures = {
        'imageTex': null,
    };

    var output = new GlObject(gl, programs_output, uniforms, attributes, textures);


    var uniforms = {
        'screenSize': uniform('vec2', null),
    };

    var attributes = {
        'posOffset': attribute(buffer, 2, gl.UNSIGNED_BYTE, false, 0, 0),
    };

    var textures = {
        'image0Tex': null,
        'image1Tex': null,
        'lightTex': null,
        'depthTex': null,
    };

    var post = new GlObject(gl, programs_post, uniforms, attributes, textures);


    return { normal: normal, sliced: sliced, output: output, post: post };
}

function build_light(gl, assets) {
    var vert = assets['light.vert'];
    var frag = assets['light.frag'];
    var programs = buildPrograms(gl, vert, frag, 1);

    var uniforms = {
        'cameraPos': uniform('vec2', null),
        'cameraSize': uniform('vec2', null),
    };

    var attributes = {
        'posOffset': attribute(null, 2, gl.BYTE, false, 16, 0),
        'center': attribute(null, 3, gl.SHORT, false, 16, 2),
        'colorAttr': attribute(null, 3, gl.UNSIGNED_BYTE, true, 16, 8),
        'radiusAttr': attribute(null, 1, gl.UNSIGNED_SHORT, false, 16, 12),
    };

    var textures = {
        'depthTex': null,
    };

    return new GlObject(gl, programs, uniforms, attributes, textures);
}

function build_structure(gl, assets, sheet_tex, depth_tex) {
    var vert = assets['structure.vert'];
    var frag = assets['structure.frag'];
    var programs = buildPrograms(gl, vert, frag, 2);

    var uniforms = {
        'sheetSize': uniform('vec2', [sheet_tex.width, sheet_tex.height]),
    };

    var attributes = {
        'position': attribute(null, 3, gl.SHORT, false, 16, 0),
        'baseZAttr': attribute(null, 1, gl.SHORT, false, 16, 6),
        'texCoord': attribute(null, 2, gl.UNSIGNED_SHORT, false, 16, 8),
    };

    var textures = {
        'sheetTex': sheet_tex,
        'depthTex': depth_tex,
    };

    return new GlObject(gl, programs, uniforms, attributes, textures);
}

Renderer.prototype._initFramebuffers = function(sw, sh) {
    this.fb_world = new Framebuffer(this.gl, sw, sh, 2);
    this.fb_light = new Framebuffer(this.gl, sw, sh, 1, false);
    this.fb_post = new Framebuffer(this.gl, sw, sh, 1, false);
    this.last_sw = sw;
    this.last_sh = sh;
};


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
    var view8 = this._asm.blockDataView8();
    var view16 = this._asm.blockDataView16();
    for (var i = 0; i < blocks.length; ++i) {
        var block = blocks[i];

        var out8 = view8.subarray(i * 14, (i + 1) * 14);
        var out16 = view16.subarray(i * 7, (i + 1) * 7);
        out16[0] = block.front;
        out16[1] = block.back;
        out16[2] = block.top;
        out16[3] = block.bottom;

        out8[8] = block.light_r;
        out8[9] = block.light_g;
        out8[10] = block.light_b;
        out16[6] = block.light_radius;
    }
};

Renderer.prototype.loadChunk = function(i, j, chunk) {
    this._asm.chunkView().set(chunk._tiles);
    this._asm.loadChunk(j, i);

    this.terrain_cache.invalidate(i * LOCAL_SIZE + j);
    var above = (i - 1) & (LOCAL_SIZE - 1);
    this.terrain_cache.invalidate(above * LOCAL_SIZE + j);
    this.sliced_cache.invalidate(above * LOCAL_SIZE + j);
};

Renderer.prototype.loadTemplateData = function(templates) {
    var view8 = this._asm.templateDataView8();
    var view16 = this._asm.templateDataView16();
    for (var i = 0; i < templates.length; ++i) {
        var template = templates[i];
        var out8 = view8.subarray(i * 22, (i + 1) * 22);
        var out16 = view16.subarray(i * 11, (i + 1) * 11);

        out8[0] = template.size.x;
        out8[1] = template.size.y;
        out8[2] = template.size.z;
        out8[3] = template.sheet;
        out16[2] = template.display_size[0];
        out16[3] = template.display_size[1];
        out16[4] = template.display_offset[0];
        out16[5] = template.display_offset[1];

        out8[12] = template.light_pos[0];
        out8[13] = template.light_pos[1];
        out8[14] = template.light_pos[2];
        out8[16] = template.light_color[0];
        out8[17] = template.light_color[1];
        out8[18] = template.light_color[2];
        out16[10] = template.light_radius;
    }
};

Renderer.prototype.addStructure = function(x, y, z, template_id) {
    var render_idx = this._asm.addStructure(x, y, z, template_id);
    var template = TemplateDef.by_id[template_id];

    var tx = (x / TILE_SIZE)|0;
    var ty = (y / TILE_SIZE)|0;
    var tz = (z / TILE_SIZE)|0;

    this._invalidateStructureRegion(tx, ty, tz, template);
    return render_idx;
};

Renderer.prototype.removeStructure = function(structure) {
    this._asm.removeStructure(structure.render_index);

    var pos = structure.pos;
    this._invalidateStructureRegion(pos.x, pos.y, pos.z, structure.template);
};

Renderer.prototype._invalidateStructureRegion = function(x, y, z, template) {
    var x0 = x;
    var x1 = x + template.size.x;

    // Avoid negative numbers
    var v0 = y - z - template.size.z + LOCAL_SIZE * CHUNK_SIZE;
    var v1 = y - z + template.size.y + LOCAL_SIZE * CHUNK_SIZE;

    var cx0 = (x0 / CHUNK_SIZE)|0;
    var cx1 = ((x1 + CHUNK_SIZE - 1) / CHUNK_SIZE)|0;
    var cv0 = (v0 / CHUNK_SIZE)|0;
    var cv1 = ((v1 + CHUNK_SIZE - 1) / CHUNK_SIZE)|0;

    var mask = LOCAL_SIZE - 1;
    for (var cy = cv0; cy < cv1; ++cy) {
        for (var cx = cx0; cx < cx1; ++cx) {
            var idx = (cy & mask) * LOCAL_SIZE + (cx & mask);
            this.terrain_cache.invalidate(idx);
            this.sliced_cache.invalidate(idx);
        }
    }
};


// Render

Renderer.prototype._renderTerrain = function(fb, cx, cy, max_z) {
    var geom = this._asm.generateTerrainGeometry(cx, cy, max_z);

    var gl = this.gl;
    gl.viewport(0, 0, fb.width, fb.height);
    gl.clearDepth(0.0);
    gl.clearColor(0, 0, 0, 0);
    gl.enable(gl.DEPTH_TEST);
    gl.depthFunc(gl.GEQUAL);

    var buffer = new Buffer(gl);
    buffer.loadData(geom);

    var this_ = this;
    fb.use(function(idx) {
        gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);

        this_.terrain_block.draw(idx, 0, geom.length / 8, {}, {
            'position': buffer,
            'texCoord': buffer,
        }, {});
    });

    gl.disable(gl.DEPTH_TEST);
};

Renderer.prototype._renderStructures = function(fb, cx, cy, max_z) {
    var gl = this.gl;
    gl.viewport(0, 0, fb.width, fb.height);
    gl.enable(gl.DEPTH_TEST);
    gl.depthFunc(gl.GEQUAL);

    this._asm.resetStructureGeometry();
    var more = true;
    while (more) {
        var result = this._asm.generateStructureGeometry(cx, cy, max_z);
        var geom = result.geometry;
        more = result.more;
        // TODO: use result.sheet

        var buffer = new Buffer(gl);
        buffer.loadData(geom);

        var this_ = this;
        fb.use(function(idx) {
            this_.structure.draw(idx, 0, geom.length / 8, {}, {
                'position': buffer,
                'baseZAttr': buffer,
                'texCoord': buffer,
            }, {});
        });
    }

    gl.disable(gl.DEPTH_TEST);
};

Renderer.prototype._renderLights = function(fb, depth_tex, cx0, cy0, cx1, cy1) {
    var gl = this.gl;
    gl.enable(gl.BLEND);
    gl.blendFunc(gl.ONE, gl.ONE);
    // clearColor sets the ambient light color+intensity
    gl.clearColor(0.2, 0.2, 0.2, 0.0);

    this._asm.resetLightGeometry(cx0, cy0, cx1, cy1);
    var more = true;
    var first = true;
    while (more) {
        var result = this._asm.generateLightGeometry();
        var geom = result.geometry;
        more = result.more;

        var buffer = new Buffer(gl);
        buffer.loadData(geom);

        var this_ = this;
        fb.use(function(idx) {
            if (first) {
                gl.clear(gl.COLOR_BUFFER_BIT);
                first = false;
            }

            if (geom.length > 0) {
                this_.light.draw(idx, 0, geom.length / 16, {}, {
                    'posOffset': buffer,
                    'center': buffer,
                    'colorAttr': buffer,
                    'radiusAttr': buffer,
                }, {
                    'depthTex': depth_tex,
                });
            }
        });
    }

    gl.disable(gl.BLEND);
};

Renderer.prototype.render = function(s, draw_extra) {
    var gl = this.gl;

    var pos = s.camera_pos;
    var size = s.camera_size;

    this.blit.setUniformValue('cameraPos', pos);
    this.blit.setUniformValue('cameraSize', size);
    this.blit_sliced.setUniformValue('cameraPos', pos);
    this.blit_sliced.setUniformValue('cameraSize', size);
    this.light.setUniformValue('cameraPos', pos);
    this.light.setUniformValue('cameraSize', size);
    // this.output uses fixed camera

    for (var k in this.sprite_classes) {
        var cls = this.sprite_classes[k];
        cls.setCamera(pos, size);
    }


    if (this.last_sw != size[0] || this.last_sh != size[1]) {
        this._initFramebuffers(size[0], size[1]);
    }


    // Populate the terrain caches.
    var cx0 = ((pos[0]|0) / CHUNK_PX)|0;
    var cx1 = (((pos[0]|0) + (size[0]|0) + CHUNK_PX) / CHUNK_PX)|0;
    var cy0 = ((pos[1]|0) / CHUNK_PX)|0;
    var cy1 = (((pos[1]|0) + (size[1]|0) + CHUNK_PX) / CHUNK_PX)|0;

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
        this_._renderTerrain(fbs.image, cx, cy, CHUNK_SIZE);
        this_._renderStructures(fbs.image, cx, cy, CHUNK_SIZE);
    });

    if (s.slice_z < CHUNK_SIZE) {
        if (s.slice_z != this.last_slice_z) {
            this.sliced_cache.invalidateAll();
        }
        this.sliced_cache.populate(chunk_idxs, function(idx, fbs) {
            var cx = (idx % LOCAL_SIZE)|0;
            var cy = (idx / LOCAL_SIZE)|0;
            this_._renderTerrain(fbs.image, cx, cy, s.slice_z);
            this_._renderStructures(fbs.image, cx, cy, s.slice_z);
        });
        this.last_slice_z = s.slice_z;
    } else {
        this.sliced_cache.reduce(0);
    }


    // Render everything into the world framebuffer.

    gl.viewport(0, 0, size[0], size[1]);
    gl.clearDepth(0.0);
    gl.clearColor(0, 0, 0, 0);
    gl.enable(gl.DEPTH_TEST);
    gl.depthFunc(gl.GEQUAL);

    this.fb_world.use(function(fb_idx) {
        gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);

        for (var cy = cy0; cy < cy1; ++cy) {
            for (var cx = cx0; cx < cx1; ++cx) {
                var idx = ((cy & (LOCAL_SIZE - 1)) * LOCAL_SIZE) + (cx & (LOCAL_SIZE - 1));

                if (s.slice_z >= CHUNK_SIZE) {
                    this_.blit.draw(fb_idx, 0, 6, {
                        'rectPos': [cx * CHUNK_SIZE * TILE_SIZE,
                                    cy * CHUNK_SIZE * TILE_SIZE],
                    }, {}, {
                        'image0Tex': this_.terrain_cache.get(idx).image.textures[0],
                        'image1Tex': this_.terrain_cache.get(idx).image.textures[1],
                        'depthTex': this_.terrain_cache.get(idx).image.depth_texture,
                    });
                } else {
                    this_.blit_sliced.draw(fb_idx, 0, 6, {
                        'rectPos': [cx * CHUNK_SIZE * TILE_SIZE,
                                    cy * CHUNK_SIZE * TILE_SIZE],
                        'sliceFrac': [s.slice_frac],
                    }, {}, {
                        'upperImage0Tex': this_.terrain_cache.get(idx).image.textures[0],
                        'upperImage1Tex': this_.terrain_cache.get(idx).image.textures[1],
                        'upperDepthTex': this_.terrain_cache.get(idx).image.depth_texture,
                        'lowerImage0Tex': this_.sliced_cache.get(idx).image.textures[0],
                        'lowerImage1Tex': this_.sliced_cache.get(idx).image.textures[1],
                        'lowerDepthTex': this_.sliced_cache.get(idx).image.depth_texture,
                    });
                }
            }
        }

        for (var i = 0; i < s.sprites.length; ++i) {
            var sprite = s.sprites[i];
            var cls = this_.sprite_classes[sprite.cls];
            if (sprite.ref_z < s.slice_z * TILE_SIZE) {
                cls.draw(fb_idx, this_, sprite, 0);
            } else {
                cls.draw(fb_idx, this_, sprite, s.slice_frac);
            }
        }
    });

    gl.disable(gl.DEPTH_TEST);


    // Render lights into the light framebuffer.

    this._renderLights(this.fb_light, this.fb_world.depth_texture,
            cx0, cy0, cx1, cy1);


    // Apply post-processing pass

    this.fb_post.use(function(idx) {
        this_.post_filter.draw(idx, 0, 6, {
            'screenSize': size,
        }, {}, {
            'image0Tex': this_.fb_world.textures[0],
            'image1Tex': this_.fb_world.textures[1],
            'lightTex': this_.fb_light.textures[0],
            'depthTex': this_.fb_world.depth_texture,
        });

        draw_extra(idx, this_);
    });


    // Copy output framebuffer to canvas.

    gl.viewport(0, 0, gl.canvas.width, gl.canvas.height);

    this.output.draw(0, 0, 6, {}, {}, {
        'imageTex': this.fb_post.textures[0],
    });
};

Renderer.prototype.renderSpecial = function(fb_idx, sprite, cls_name) {
    var cls = this.sprite_classes[cls_name];
    cls.draw(fb_idx, this, sprite, 0);
};



/** @constructor */
function RenderCache(gl, init) {
    this.gl = gl;
    this.init = init;

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
    this.slots.push(this.init(this.gl));
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

RenderCache.prototype.invalidateAll = function() {
    for (var slot = 0; slot < this.slots.length; ++slot) {
        var idx = this.users[slot];
        if (idx != -1) {
            this.map[idx] = -1;
        }
        this.users[slot] = -1;
    }
};

RenderCache.prototype.reduce = function(len) {
    for (var i = len; i < this.users.length; ++i) {
        var idx = this.users[i];
        this.map[idx] = -1;
    }
    for (var i = 0; i < len; ++i) {
        this.slots.pop();
        this.users.pop();
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
