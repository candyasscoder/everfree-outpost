var Asm = require('asmlibs').Asm;
var getRendererHeapSize = require('asmlibs').getRendererHeapSize;
var OffscreenContext = require('canvas').OffscreenContext;
var TileDef = require('chunk').TileDef;
var CHUNK_SIZE = require('chunk').CHUNK_SIZE;
var TILE_SIZE = require('chunk').TILE_SIZE;
var LOCAL_SIZE = require('chunk').LOCAL_SIZE;
var Program = require('glutil').Program;
var Texture = require('glutil').Texture;
var Buffer = require('glutil').Buffer;

var GlObject = require('glutil').GlObject;
var uniform = require('glutil').uniform;
var attribute = require('glutil').attribute;


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
        'atlasSize': uniform('float',
                [(atlas.width / TILE_SIZE)|0,
                 (atlas.height / TILE_SIZE)|0]),
        'cameraPos': uniform('float', null),
        'cameraSize': uniform('float', null),
        'chunkPos': uniform('float', null),
    };

    var terrain_attributes = {
        'position': attribute(null, 2, gl.UNSIGNED_BYTE, false, 4, 0),
        'texCoord': attribute(null, 2, gl.UNSIGNED_BYTE, false, 4, 2),
    };

    this.terrain_obj = new GlObject(gl, terrain_program,
            terrain_uniforms,
            terrain_attributes,
            {'atlasSampler': atlas_texture});


    var sprite_vert = assets['sprite.vert'];
    var sprite_frag = assets['sprite.frag'];
    var sprite_program = new Program(gl, sprite_vert, sprite_frag);

    var sprite_buffer = new Buffer(gl);
    sprite_buffer.loadData(new Uint8Array([
            0, 0,
            0, 1,
            1, 1,

            0, 0,
            1, 1,
            1, 0,
    ]));

    var sprite_uniforms = {
        'cameraPos': uniform('float', null),
        'cameraSize': uniform('float', null),
        'sheetSize': uniform('float', null),
        'base': uniform('float', null),
        'off': uniform('float', null),
        'size': uniform('float', null),
        'flip': uniform('float', null),
    };
    this.sprite_obj = new GlObject(gl, sprite_program,
            sprite_uniforms,
            {'position': attribute(sprite_buffer, 2, gl.UNSIGNED_BYTE, false, 0, 0)},
            {'sheetSampler': new Texture(gl)});
};

Renderer.prototype.setSpriteSheet = function(sheet) {
    var img = sheet.image;
    this.sprite_obj.getTexture('sheetSampler').loadImage(img);
    this.sprite_obj.setUniformValue('sheetSize', [img.width, img.height]);
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

Renderer.prototype.render = function(ctx, sx, sy, sw, sh, sprites) {
    var gl = this.gl;

    this.terrain_obj.setUniformValue('cameraPos', [sx, sy]);
    this.terrain_obj.setUniformValue('cameraSize', [sw, sh]);

    this.sprite_obj.setUniformValue('cameraPos', [sx, sy]);
    this.sprite_obj.setUniformValue('cameraSize', [sw, sh]);

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
        // lie within some other region.  clip_off is the offset of x,y,w,h
        // within the sprite's normal region.  The source offsets are adjusted
        // by this amount to grab the right part of the image for x,y,w,h.
        var clip_off_x = x - x0;
        var clip_off_y = y - y0;
        // If the sprite is flipped, we need to flip the offset.  We draw the
        // left half of a flipped sprite by drawing a flipped version of the
        // right half of the source.
        if (sprite.flip) {
            clip_off_x = sprite.width - clip_off_x - w;
        }

        var uniforms = {
            'base': [x, y],
            'off': [sprite.offset_x + clip_off_x,
                    sprite.offset_y + clip_off_y],
            'size': [w, h],
            'flip': [sprite.flip, 0],
        };

        this_.sprite_obj.draw(0, 6, uniforms, {}, {});
    }

    this._asm.render(sx, sy, sw, sh, sprites, buffer_terrain, draw_sprite);

    flush_terrain();
};


/** @constructor */
function Sprite() {
    this.image = null;
    this.offset_x = 0;
    this.offset_y = 0;
    this.width = 0;
    this.height = 0;
    this.flip = false;

    this.ref_x = 0;
    this.ref_y = 0;
    this.ref_z = 0;
    this.anchor_x = 0;
    this.anchor_y = 0;
}
exports.Sprite = Sprite;

Sprite.prototype.refPosition = function() {
    return new Vec(this.ref_x, this.ref_y, this.ref_z);
};

Sprite.prototype.setSource = function(image, offset_x, offset_y, width, height) {
    this.image = image;
    this.offset_x = offset_x;
    this.offset_y = offset_y;
    this.width = width;
    this.height = height;
};

Sprite.prototype.setFlip = function(flip) {
    this.flip = flip;
};

Sprite.prototype.setDestination = function(ref_pos, anchor_x, anchor_y) {
    this.ref_x = ref_pos.x;
    this.ref_y = ref_pos.y;
    this.ref_z = ref_pos.z;
    this.anchor_x = anchor_x;
    this.anchor_y = anchor_y;
};
