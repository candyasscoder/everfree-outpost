var Asm = require('asmlibs').Asm;
var SIZEOF = require('asmlibs').SIZEOF;
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
var makeShaders = require('graphics/shaders').makeShaders;
var ChunkRenderer = require('graphics/chunk').ChunkRenderer;

var GlObject = require('graphics/glutil').GlObject;
var uniform = require('graphics/glutil').uniform;
var attribute = require('graphics/glutil').attribute;

//var Simple3D = require('graphics/draw/simple').Simple3D;
//var Layered3D = require('graphics/draw/layered').Layered3D;
//var Named3D = require('graphics/draw/named').Named3D;
//var PonyOutline3D = require('graphics/draw/ponyoutline').PonyOutline3D;
var PonyAppearanceClass = require('graphics/appearance/pony').PonyAppearanceClass;

var CHUNK_PX = CHUNK_SIZE * TILE_SIZE;

// The `now` value passed to the animation shader must be reduced to fit in a
// float.  We use the magic number 55440 for this, since it's divisible by
// every number from 1 to 12 (and most "reasonable" numbers above that).  This
// is useful because repeating animations will glitch when `now` wraps around
// unless `length / framerate` divides evenly into the modulus.
//
// Note that the shader `now` and ANIM_MODULUS are both in seconds, not ms.
var ANIM_MODULUS = 55440;

// We also need a smaller modulus for one-shot animation start times.  These
// are measured in milliseconds and must fit in a 16-bit int.  It's important
// that the one-shot modulus divides evenly into 1000 * ANIM_MODULUS, because
// the current frame time in milliseconds will be modded by 1000 * ANIM_MODULUS
// and then again by the one-shot modulus.
//
// We re-use ANIM_MODULUS as the one-shot modulus, since it obviously divides
// evenly into 1000 * ANIM_MODULUS.  This is okay as long as ANIM_MODULUS fits
// into 16 bits.
var ONESHOT_MODULUS = ANIM_MODULUS;


/** @constructor */
function Renderer(gl) {
    this.gl = gl;
    this._asm = new Asm(getGraphicsHeapSize());
    this._asm.initStructureBuffer();
    this._asm.initLightState();

    this.texture_cache = new WeakMap();
    this.chunk_cache = new RenderCache();
}
exports.Renderer = Renderer;


// Renderer initialization

Renderer.prototype.initGl = function(assets) {
    var gl = this.gl;

    var this_ = this;
    makeShaders(this, gl, assets, function(img) { return this_.cacheTexture(img); });

    this.classes = {
        pony: new PonyAppearanceClass(gl, assets),
    };

    this.last_sw = -1;
    this.last_sh = -1;

    // Temporary framebuffer for storing shadows and other translucent parts
    // during structure rendering.  This doesn't depend on the screen size,
    // which is why it's not in _initFramebuffers with the rest.
    this.fb_shadow = new Framebuffer(this.gl, CHUNK_PX, CHUNK_PX, 1);
};

Renderer.prototype._initFramebuffers = function(sw, sh) {
    // Framebuffer containing image and metadata for the world (terrain +
    // structures).
    this.fb_world = new Framebuffer(this.gl, sw, sh, 2);
    // Framebuffer containing light intensity at every pixel.
    this.fb_light = new Framebuffer(this.gl, sw, sh, 1, false);
    // Framebuffer containing postprocessed image data.  This is emitted
    // directly to the screen.  (May require upscaling, which is why the
    // postprocessing shader doesn't output to the screen immediately.)
    this.fb_post = new Framebuffer(this.gl, sw, sh, 1, false);

    // this.fb_shadow does not depend on sw/sh, so it gets initialized
    // elsewhere.

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

// Helper function for writing data into an asm structure.  Constructs a
// subarray of `view` for accessing element `index` in an array of structures
// of the given `size`.  The `size` should include any necessary padding for
// alignment following each structure.
function mk_out(view, index, size) {
    var shift;
    switch (view.constructor.BYTES_PER_ELEMENT) {
        case 1: shift = 0; break;
        case 2: shift = 1; break;
        case 4: shift = 2; break;
        case 8: shift = 3; break;
        default: throw 'TypedArray has non-power-of-two BYTES_PER_ELEMENT';
    }
    var arr = view.subarray(index * (size >> shift), (index + 1) * (size >> shift));

    // If `count` is null, store number `x` at byte offset `j`.  Otherwise,
    // store `count` numbers from array `x` starting at byte offset `j`.
    return function(j, x, count) {
        if (count == null) {
            arr[j >> shift] = x;
        } else {
            for (var k = 0; k < count; ++k) {
                arr[(j >> shift) + k] = x[k];
            }
        }
    };
}

Renderer.prototype.loadBlockData = function(blocks) {
    var view8 = this._asm.blockDataView8();
    var view16 = this._asm.blockDataView16();
    for (var i = 0; i < blocks.length; ++i) {
        var block = blocks[i];
        var out8 = mk_out(view8, i, SIZEOF.BlockDisplay);
        var out16 = mk_out(view16, i, SIZEOF.BlockDisplay);

        out16(  0, block.front);
        out16(  2, block.back);
        out16(  4, block.top);
        out16(  6, block.bottom);

        out8(   8, block.light_color, 3);
        out16( 12, block.light_radius);
    }
};

Renderer.prototype.loadChunk = function(i, j, chunk) {
    this._asm.chunkView().set(chunk._tiles);
    this._asm.loadChunk(j, i);

    this.chunk_cache.ifPresent(i * LOCAL_SIZE + j, function(cr) {
        cr.invalidateTerrain();
    });

    var above = (i - 1) & (LOCAL_SIZE - 1);
    this.chunk_cache.ifPresent(above * LOCAL_SIZE + j, function(cr) {
        cr.invalidateTerrain();
    });
};

Renderer.prototype.loadTemplateData = function(templates) {
    var view8 = this._asm.templateDataView8();
    var view16 = this._asm.templateDataView16();

    for (var i = 0; i < templates.length; ++i) {
        var template = templates[i];
        var out8 = mk_out(view8, i, SIZEOF.StructureTemplate);
        var out16 = mk_out(view16, i, SIZEOF.StructureTemplate);

        out8(   0, template.size.x);
        out8(   1, template.size.y);
        out8(   2, template.size.z);
        out8(   3, template.sheet);
        out16(  4, template.display_size, 2);
        out16(  8, template.display_offset, 2);
        out8(  12, template.layer);

        out8(  13, template.anim_sheet);
        var oneshot_length = template.anim_length * (template.anim_oneshot ? -1 : 1);
        out8(  14, oneshot_length);
        out8(  15, template.anim_rate);
        out16( 16, template.anim_offset, 2);
        out16( 20, template.anim_pos, 2);
        out8(  24, template.anim_size, 2);

        out8(  26, template.light_pos, 3);
        out8(  29, template.light_color, 3);
        out16( 32, template.light_radius);
    }
};

Renderer.prototype.addStructure = function(now, x, y, z, template) {
    var render_idx = this._asm.addStructure(x, y, z, template.id);
    if (template.anim_oneshot) {
        // The template defines a one-shot animation.  Set the start time to
        // now.
        this._asm.setStructureOneshotStart(render_idx, now % ONESHOT_MODULUS);
    }

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
            this.chunk_cache.ifPresent(idx, function(cr) {
                cr.invalidateStructures();
            });
        }
    }
};


// Render
// This section has screen-space passes only.  Other rendering is done by
// ChunkRenderer in graphics/chunk.js.

Renderer.prototype._renderStaticLights = function(fb, depth_tex, cx0, cy0, cx1, cy1, amb) {
    var gl = this.gl;
    gl.enable(gl.BLEND);
    gl.blendFunc(gl.ONE, gl.ONE);
    // clearColor sets the ambient light color+intensity
    var amb_intensity = 0.2126 * amb[0] + 0.7152 * amb[1] + 0.0722 * amb[2];
    gl.clearColor(amb[0] / 255, amb[1] / 255, amb[2] / 255, amb_intensity / 255);

    fb.use(function(idx) {
        gl.clear(gl.COLOR_BUFFER_BIT);
    });

    this._asm.resetLightGeometry(cx0, cy0, cx1, cy1);
    var more = true;
    while (more) {
        var result = this._asm.generateLightGeometry();
        var geom = result.geometry;
        more = result.more;

        var buffer = new Buffer(gl);
        buffer.loadData(geom);

        var this_ = this;
        fb.use(function(idx) {
            if (geom.length > 0) {
                this_.static_light.draw(idx, 0, geom.length / SIZEOF.LightVertex,
                        {}, {'*': buffer}, {'depthTex': depth_tex});
            }
        });
    }

    gl.disable(gl.BLEND);
};

Renderer.prototype._renderDynamicLights = function(fb, depth_tex, lights) {
    var gl = this.gl;
    gl.enable(gl.BLEND);
    gl.blendFunc(gl.ONE, gl.ONE);

    var this_ = this;
    fb.use(function(idx) {
        for (var i = 0; i < lights.length; ++i) {
            var light = lights[i];
            this_.dynamic_light.draw(idx, 0, 6, {
                'center': [
                    light.pos.x,
                    light.pos.y,
                    light.pos.z,
                ],
                'colorIn': [
                    light.color[0] / 255,
                    light.color[1] / 255,
                    light.color[2] / 255,
                ],
                'radiusIn': [light.radius],
            }, {}, {
                'depthTex': depth_tex,
            });
        }
    });

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
    this.static_light.setUniformValue('cameraPos', pos);
    this.static_light.setUniformValue('cameraSize', size);
    this.dynamic_light.setUniformValue('cameraPos', pos);
    this.dynamic_light.setUniformValue('cameraSize', size);
    // this.blit_full uses fixed camera

    for (var k in this.classes) {
        var cls = this.classes[k];
        cls.setCamera(pos, size);
    }

    this.structure_anim.setUniformValue('now', [s.now / 1000 % ANIM_MODULUS]);

    this.blit_sliced.setUniformValue('sliceFrac', [s.slice_frac]);


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
    this.chunk_cache.populate(chunk_idxs, function(idx) {
        var cx = (idx % LOCAL_SIZE)|0;
        var cy = (idx / LOCAL_SIZE)|0;
        return new ChunkRenderer(this_, cx, cy);
    });
    this.chunk_cache.forEach(function(cr) {
        cr.setSliceZ(s.slice_z);
        cr.update();
    });


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
                this_.chunk_cache.get(idx).draw(fb_idx, cx, cy);
            }
        }

        for (var i = 0; i < s.sprites.length; ++i) {
            var sprite = s.sprites[i];
            if (sprite.ref_z < s.slice_z * TILE_SIZE) {
                sprite.appearance.draw3D(fb_idx, this_, sprite, 0);
            } else {
                sprite.appearance.draw3D(fb_idx, this_, sprite, s.slice_frac);
            }
        }
    });

    gl.disable(gl.DEPTH_TEST);


    // Render lights into the light framebuffer.

    this._renderStaticLights(this.fb_light, this.fb_world.depth_texture,
            cx0, cy0, cx1, cy1,
            s.ambient_color);

    this._renderDynamicLights(this.fb_light, this.fb_world.depth_texture,
            s.lights);


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

    this.blit_full.draw(0, 0, 6, {}, {}, {
        'imageTex': this.fb_post.textures[0],
    });
};

Renderer.prototype.renderSpecial = function(fb_idx, sprite) {
    sprite.appearance.draw3D(fb_idx, this, sprite, 0);
};



/** @constructor */
function RenderCache() {
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
    this.slots.push(null);
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
            this.slots[free] = callback(idx);
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

RenderCache.prototype.ifPresent = function(idx, callback) {
    var slot = this.map[idx];
    if (slot != -1 && this.users[slot] == idx) {
        callback(this.slots[slot]);
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

RenderCache.prototype.forEach = function(callback) {
    for (var i = 0; i < this.slots.length; ++i) {
        callback(this.slots[i], this.users[i]);
    }
};
