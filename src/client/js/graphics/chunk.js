var CHUNK_SIZE = require('data/chunk').CHUNK_SIZE;
var TILE_SIZE = require('data/chunk').TILE_SIZE;
var LOCAL_SIZE = require('data/chunk').LOCAL_SIZE;

var CHUNK_PX = CHUNK_SIZE * TILE_SIZE;

var SIZEOF = require('asmlibs').SIZEOF;


function ChunkRenderer(r, cx, cy) {
    this.r = r;
    this.cx = cx;
    this.cy = cy;

    this.base_fb = null;
    this.shadow_fb = null;
    this.normal_fbs_dirty = false;

    this.anim_data = null;
    this.anim_data_dirty = false;

    this.sliced_base_fb = null;
    this.sliced_shadow_fb = null;
    this.slice_z = CHUNK_SIZE;
    this.sliced_fbs_dirty = false;
}
exports.ChunkRenderer = ChunkRenderer;


ChunkRenderer.prototype.invalidateTerrain = function() {
    this.normal_fbs_dirty = true;
    this.sliced_fbs_dirty = true;
};

ChunkRenderer.prototype.invalidateStructures = function() {
    this.normal_fbs_dirty = true;
    this.sliced_fbs_dirty = true;
    this.anim_data_dirty = true;
};

ChunkRenderer.prototype.setSliceZ = function(slice_z) {
    if (this.slice_z == slice_z) {
        return;
    }

    this.slice_z = slice_z;
    if (this._slicingEnabled()) {
        this.sliced_fbs_dirty = true;
    } else {
        this.sliced_base_fb = null;
        this.sliced_shadow_fb = null;
    }
};

ChunkRenderer.prototype._slicingEnabled = function() {
    return this.slice_z < CHUNK_SIZE;
};


ChunkRenderer.prototype._updateNormalFbs = function() {
    var dirty = this.normal_fbs_dirty;
    if (this.base_fb == null) {
        this.base_fb = new Framebuffer(this.r.gl, CHUNK_PX, CHUNK_PX, 2);
        dirty = true;
    }
    if (this.shadow_fb == null) {
        this.shadow_fb = new Framebuffer(this.r.gl, CHUNK_PX, CHUNK_PX, 1);
        dirty = true;
    }
    if (dirty) {
        this._renderTerrain(this.base_fb, CHUNK_SIZE);
        this._renderStructures(this.base_fb, this.shadow_fb, CHUNK_SIZE);
        this.normal_fbs_dirty = false;
    }
};

ChunkRenderer.prototype._updateSlicedFbs = function() {
    if (!this._slicingEnabled()) {
        return;
    }

    var dirty = this.sliced_fbs_dirty;
    if (this.sliced_base_fb == null) {
        this.sliced_base_fb = new Framebuffer(this.r.gl, CHUNK_PX, CHUNK_PX, 2);
        dirty = true;
    }
    if (this.sliced_shadow_fb == null) {
        this.sliced_shadow_fb = new Framebuffer(this.r.gl, CHUNK_PX, CHUNK_PX, 1);
        dirty = true;
    }
    if (dirty) {
        this._renderTerrain(this.sliced_base_fb, this.slice_z);
        this._renderStructures(this.sliced_base_fb, this.sliced_shadow_fb, this.slice_z);
        this.sliced_fbs_dirty = false;
    }
};

ChunkRenderer.prototype._updateAnimData = function() {
    var dirty = this.anim_data_dirty;
    if (this.anim_data == null) {
        this.anim_data = new Buffer(this.r.gl);
        dirty = true;
    }
    if (dirty) {
        this._initAnimData(this.anim_data);
        this.anim_data_dirty = false;
    }
};

// `update` is separate from `draw` because `draw` is called while there is
// already a framebuffer bound (since it's drawing into that framebuffer), so
// we can't `update` then without additional work to restore the previous
// framebuffer binding.
ChunkRenderer.prototype.update = function() {
    this._updateNormalFbs();
    this._updateSlicedFbs();
    this._updateAnimData();
};


ChunkRenderer.prototype._renderTerrain = function(fb, slice_z) {
    var r = this.r;
    var gl = this.r.gl;
    gl.viewport(0, 0, fb.width, fb.height);
    gl.clearDepth(0.0);
    gl.clearColor(0, 0, 0, 0);
    gl.enable(gl.DEPTH_TEST);
    gl.depthFunc(gl.GEQUAL);

    var geom = r._asm.generateTerrainGeometry(this.cx, this.cy, slice_z);
    var buffer = new Buffer(gl);
    buffer.loadData(geom);

    fb.use(function(idx) {
        gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT);
        r.terrain_block.draw(idx, 0, geom.length / SIZEOF.TerrainVertex,
                {}, {'*': buffer}, {});
    });

    gl.disable(gl.DEPTH_TEST);
};

ChunkRenderer.prototype._renderStructures = function(fb_image, fb_shadow, slice_z) {
    var r = this.r;
    var gl = this.r.gl;

    gl.viewport(0, 0, fb_image.width, fb_image.height);
    gl.enable(gl.DEPTH_TEST);
    gl.depthFunc(gl.ALWAYS);

    // Copy the terrain depth buffer from fb_image over to fb_shadow.
    // Otherwise the shadow rendering pass would have no way of knowing what
    // shadows are occluded by terrain.
    fb_shadow.use(function(idx) {
        r.blit_depth.draw(idx, 0, 6, {}, {}, {'depthTex': fb_image.depth_texture});
    });

    gl.depthFunc(gl.GEQUAL);

    r._asm.resetStructureGeometry();
    var more = true;
    while (more) {
        var result = r._asm.generateStructureGeometry(this.cx, this.cy, slice_z);
        var geom = result.geometry;
        more = result.more;
        // TODO: use result.sheet

        var buffer = new Buffer(gl);
        buffer.loadData(geom);

        // Render images and metadata.
        fb_image.use(function(idx) {
            r.structure.draw(idx, 0, geom.length / SIZEOF.StructureVertex,
                    {}, {'*': buffer}, {});
        });

        // Render shadows only.
        fb_shadow.use(function(idx) {
            r.structure_shadow.draw(idx, 0, geom.length / SIZEOF.StructureVertex,
                    {}, {'*': buffer}, {});
        });
    }

    gl.disable(gl.DEPTH_TEST);
};

ChunkRenderer.prototype._initAnimData = function(buf) {
    var r = this.r;

    var geom_parts = [];
    var geom_len = 0;
    r._asm.resetStructureGeometry();
    var more = true;
    while (more) {
        var result = r._asm.generateStructureAnimGeometry(this.cx, this.cy, CHUNK_SIZE);
        var geom = result.geometry;
        more = result.more;
        // TODO: use result.sheet

        geom_parts.push(geom);
        geom_len += geom.length;
    }

    var all_geom = new Uint8Array(geom_len);
    var offset = 0;
    for (var i = 0; i < geom_parts.length; ++i) {
        all_geom.set(geom_parts[i], offset);
        offset += geom_parts[i].length;
    }

    buf.loadData(all_geom);
};


ChunkRenderer.prototype._doBlit = function(idx, draw_cx, draw_cy, normal_fb, sliced_fb) {
    var r = this.r;
    var gl = this.r.gl;

    if (!this._slicingEnabled()) {
        r.blit.draw(idx, 0, 6, {
            'rectPos': [draw_cx * CHUNK_PX, draw_cy * CHUNK_PX],
        }, {}, {
            'image0Tex': normal_fb.textures[0],
            'image1Tex': normal_fb.textures[1],
            'depthTex': normal_fb.depth_texture,
        });
    } else {
        r.blit_sliced.draw(idx, 0, 6, {
            'rectPos': [draw_cx * CHUNK_PX, draw_cy * CHUNK_PX],
        }, {}, {
            'upperImage0Tex': normal_fb.textures[0],
            'upperImage1Tex': normal_fb.textures[1],
            'upperDepthTex': normal_fb.depth_texture,
            'lowerImage0Tex': sliced_base_fb.textures[0],
            'lowerImage1Tex': sliced_base_fb.textures[1],
            'lowerDepthTex': sliced_base_fb.depth_texture,
        });
    }
};

ChunkRenderer.prototype.draw = function(idx, draw_cx, draw_cy) {
    var r = this.r;
    var gl = this.r.gl;

    this._doBlit(idx, draw_cx, draw_cy, this.base_fb, this.sliced_base_fb);

    if (idx == 0) {
        // Composite shadows over the rest.
        gl.enable(gl.BLEND);
        gl.blendFunc(gl.SRC_ALPHA, gl.ONE_MINUS_SRC_ALPHA);

        this._doBlit(idx, draw_cx, draw_cy, this.shadow_fb, this.sliced_shadow_fb);

        gl.disable(gl.BLEND);
    }
};
