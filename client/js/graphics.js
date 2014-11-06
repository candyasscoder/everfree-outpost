var OffscreenContext = require('canvas').OffscreenContext;
var TileDef = require('chunk').TileDef;
var CHUNK_SIZE = require('chunk').CHUNK_SIZE;
var TILE_SIZE = require('chunk').TILE_SIZE;
var LOCAL_SIZE = require('chunk').LOCAL_SIZE;


/** @constructor */
function Renderer(tile_sheet) {
    this.tile_sheet = tile_sheet;

    var chunk_total = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;
    var local_total = LOCAL_SIZE * LOCAL_SIZE;

    var this_ = this;
    function handler(src, sx, sy, dst, dx, dy, w, h) {
        this_._renderCallback(src, sx, sy, dst, dx, dy, w, h);
    }
    this._asm = new Asm(Asm.getRendererHeapSize(), handler);

    this._chunks = [];
    this._chunk_images = [];
    for (var i = 0; i < LOCAL_SIZE * LOCAL_SIZE; ++i) {
        this._chunk_images[i] = null;
    }
}
exports.Renderer = Renderer;


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

    this._chunks[idx] = chunk;
    if (this._chunk_images[idx] == null) {
        var width = CHUNK_SIZE * TILE_SIZE;
        var height = CHUNK_SIZE * TILE_SIZE * 2;
        this._chunk_images[idx] = new OffscreenContext(width, height);
    }
    this._renderChunkImage(chunk, this._chunk_images[idx]);

    this._asm.chunkDataView().set(chunk._tiles);
    i = (idx / LOCAL_SIZE)|0;
    j = (idx % LOCAL_SIZE);
    this._asm.updateXvData(i, j);
};

Renderer.prototype._renderChunkImage = function(chunk, ctx) {
    ctx.clearRect(0, 0, ctx.canvas.width, ctx.canvas.height);
    var sheet = this.tile_sheet;
    function maybe_draw(display, sx, sy) {
        if (display == 0) {
            return;
        }
        sheet.drawInto(ctx, display >> 5, display & 0x1f, sx, sy);
    }

    for (var z = 0; z < CHUNK_SIZE; ++z) {
        for (var y = 0; y < CHUNK_SIZE; ++y) {
            for (var x = 0; x < CHUNK_SIZE; ++x) {
                var sx = x * TILE_SIZE;
                var sy_offset = (y - z) * TILE_SIZE;
                var sy1 = sy_offset + CHUNK_SIZE * TILE_SIZE;
                var sy0 = sy1 - TILE_SIZE;

                var tile = chunk.get(x, y, z);
                maybe_draw(tile.back, sx, sy0);
                maybe_draw(tile.bottom, sx, sy1);
                maybe_draw(tile.top, sx, sy0);
                maybe_draw(tile.front, sx, sy1);
            }
        }
    }
};

Renderer.prototype._renderCallback = function(src, sx, sy, dst, dx, dy, w, h) {
    var src_img;
    var flip = false;
    if (src == 0) {
        src_img = null;
    } else if (src == 1) {
        src_img = this._cur_ctx.canvas;
    } else if (src == 2) {
        src_img = this.tile_sheet.image;
    } else if (8 <= src && src < 64) {
        console.assert(false, 'render caches are not supported yet');
    } else if (64 <= src && src < 128) {
        var img_ctx = this._chunk_images[src - 64];
        src_img = img_ctx != null ? img_ctx.canvas : null;
    } else if (128 <= src) {
        var sprite = this._cur_sprites[src - 128];
        src_img = sprite.image;
        sx += sprite.offset_x;
        sy += sprite.offset_y;
        flip = sprite.flip;
    } else {
        console.assert(false, 'bad source ID', src);
    }

    var dst_ctx;
    if (dst == 0) {
        return;
    } else if (dst == 1) {
        dst_ctx = this._cur_ctx;
    } else if (dst == 2) {
        console.assert(false, 'tile atlas is read-only');
    } else if (8 <= dst && dst < 64) {
        console.assert(false, 'render caches are not supported yet');
    } else if (64 <= dst && dst < 128) {
        dst_ctx = this._chunk_images[dst - 64];
    } else if (128 <= dst) {
        console.assert(false, 'sprite images are read-only');
    } else {
        console.assert(false, 'bad destination ID', dst);
    }

    if (dst_ctx == null) {
        return;
    }

    if (src_img == null) {
        dst_ctx.clearRect(dx, dy, w, h);
    } else {
        if (flip) {
            dst_ctx.save();
            dst_ctx.scale(-1, 1);
            dx = -dx - w;
        }
        dst_ctx.drawImage(src_img, sx, sy, w, h, dx, dy, w, h);
        dst_ctx.strokeRect(dx, dy, w, h);
        if (flip) {
            dst_ctx.restore();
        }
    }
};

Renderer.prototype.render = function(ctx, sx, sy, sw, sh, sprites) {
    this._cur_ctx = ctx;
    this._cur_sprites = sprites;
    this._asm.render(sx, sy, sw, sh, sprites);
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
