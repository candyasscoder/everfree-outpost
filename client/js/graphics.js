var Asm = require('asmlibs').Asm;
var OffscreenContext = require('canvas').OffscreenContext;
var TileDef = require('chunk').TileDef;
var CHUNK_SIZE = require('chunk').CHUNK_SIZE;
var TILE_SIZE = require('chunk').TILE_SIZE;
var LOCAL_SIZE = require('chunk').LOCAL_SIZE;


var HAS_TOP     = 0x01;
var HAS_BOTTOM  = 0x02;
var HAS_FRONT   = 0x04;
var HAS_BACK    = 0x08;

/** @constructor */
function TerrainGraphics(sheet) {
    this.sheet = sheet;

    var chunk_total = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;
    this._asm = new Asm(chunk_total * 9);

    var local_total = LOCAL_SIZE * LOCAL_SIZE;
    this._chunks = [];
    for (var i = 0; i < local_total; ++i) {
        this._chunks.push(null);
    }
}
exports.TerrainGraphics = TerrainGraphics;

TerrainGraphics.prototype.loadChunk = function(ci, cj, tiles) {
    var view = this._asm.chunkFlagsView();
    console.assert(tiles.length == view.length,
            'expected ' + view.length + ' tiles, but got ' + tiles.length);

    for (var i = 0; i < tiles.length; ++i) {
        var tile = TileDef.by_id[tiles[i]];
        var flags = 0;
        if (tile.front != 0) {
            flags |= HAS_FRONT;
        }
        if (tile.back != 0) {
            flags |= HAS_BACK;
        }
        if (tile.top != 0) {
            flags |= HAS_TOP;
        }
        if (tile.bottom != 0) {
            flags |= HAS_BOTTOM;
        }
        view[i] = flags;
    }

    var result = this._asm.bakeChunk();
    this._chunks[ci * LOCAL_SIZE + cj] = new ChunkGraphics(tiles, result, this.sheet);
};

TerrainGraphics.prototype.unloadChunk = function(ci, cj) {
    this._chunks[ci * LOCAL_SIZE * cj] = null;
};

TerrainGraphics.prototype.render = function(ctx, ci, cj, sprites) {
    var chunk = this._chunks[ci * LOCAL_SIZE + cj];
    if (chunk != null) {
        chunk.render(ctx, sprites);
    }
};


var PAGE_WIDTH = 16;
var PAGE_HEIGHT = 32;

/** @constructor */
function ChunkGraphics(tiles, bake_result, sheet) {
    this._tiles = tiles;

    this._pages = [];
    for (var i = 0; i < bake_result.pages; ++i) {
        this._pages.push(new OffscreenContext(PAGE_WIDTH * TILE_SIZE, PAGE_HEIGHT * TILE_SIZE));
    }

    this._layers = [];
    for (var i = 0; i < bake_result.layers.length; ++i) {
        var layer = this._initLayer(bake_result.layers[i], sheet);
        this._layers.push(layer);
    }
    this._layers.sort(function(a, b) {
        if (a.pos_u != b.pos_u) {
            return a.pos_u - b.pos_u;
        } else if (a.pos_v != b.pos_v) {
            return a.pos_v - b.pos_v;
        } else {
            return a.pos_x - b.pos_x;
        }
    });
}

ChunkGraphics.prototype._initLayer = function(layer, sheet) {
    var page = this._pages[layer.page];
    var horiz = layer.min.z == layer.max.z;

    var height;
    if (horiz) {
        this._initLayerHoriz(layer, page, sheet);
        height = layer.max.y - layer.min.y;
    } else {
        this._initLayerVert(layer, page, sheet);
        height = layer.max.z - layer.min.z;
    }

    return ({
        page: layer.page,

        src_x: TILE_SIZE * layer.pos_x,
        src_y: TILE_SIZE * layer.pos_y,
        src_w: TILE_SIZE * (layer.max.x - layer.min.x),
        src_h: TILE_SIZE * height,

        // Output position in XUV space, used to determine ordering.
        pos_x: TILE_SIZE * layer.min.x,
        pos_u: TILE_SIZE * (layer.min.y + layer.min.z),
        pos_v: TILE_SIZE * (layer.min.y - layer.min.z),

        // Y-coordinate to use when rendering.
        dst_y: TILE_SIZE * (layer.min.y - layer.min.z - (horiz ? 0 : height)),
    });
};

ChunkGraphics.prototype._initLayerHoriz = function(layer, page, sheet) {
    var z = layer.min.z;

    if (z > 0) {
        for (var y = layer.min.y; y < layer.max.y; ++y) {
            for (var x = layer.min.x; x < layer.max.x; ++x) {
                var idx = ((z - 1) * CHUNK_SIZE + y) * CHUNK_SIZE + x;
                var image = TileDef.by_id[this._tiles[idx]].top;

                var x_out = x - layer.min.x + layer.pos_x;
                var y_out = y - layer.min.y + layer.pos_y;

                sheet.drawInto(page, image >> 4, image & 0xf,
                        x_out * TILE_SIZE, y_out * TILE_SIZE);
            }
        }
    }

    if (z < CHUNK_SIZE) {
        for (var y = layer.min.y; y < layer.max.y; ++y) {
            for (var x = layer.min.x; x < layer.max.x; ++x) {
                var idx = (z * CHUNK_SIZE + y) * CHUNK_SIZE + x;
                var image = TileDef.by_id[this._tiles[idx]].bottom;

                var x_out = x - layer.min.x + layer.pos_x;
                var y_out = y - layer.min.y + layer.pos_y;

                sheet.drawInto(page, image >> 4, image & 0xf,
                        x_out * TILE_SIZE, y_out * TILE_SIZE);
            }
        }
    }
};

ChunkGraphics.prototype._initLayerVert = function(layer, page, sheet) {
    var y = layer.min.y;

    if (y > 0) {
        for (var z = layer.min.z; z < layer.max.z; ++z) {
            for (var x = layer.min.x; x < layer.max.x; ++x) {
                var idx = (z * CHUNK_SIZE + (y - 1)) * CHUNK_SIZE + x;
                var image = TileDef.by_id[this._tiles[idx]].front;

                var x_out = x - layer.min.x + layer.pos_x;
                var y_out = layer.max.z - 1 - z + layer.pos_y;

                sheet.drawInto(page, image >> 4, image & 0xf,
                        x_out * TILE_SIZE, y_out * TILE_SIZE);
            }
        }
    }

    if (y < CHUNK_SIZE) {
        for (var z = layer.min.z; z < layer.max.z; ++z) {
            for (var x = layer.min.x; x < layer.max.x; ++x) {
                var idx = (z * CHUNK_SIZE + y) * CHUNK_SIZE + x;
                var image = TileDef.by_id[this._tiles[idx]].front;

                var x_out = x - layer.min.x + layer.pos_x;
                var y_out = layer.max.z - 1 - z + layer.pos_y;

                sheet.drawInto(page, image >> 4, image & 0xf,
                        x_out * TILE_SIZE, y_out * TILE_SIZE);
            }
        }
    }
};

ChunkGraphics.prototype.render = function(ctx, sprites) {
    var layers = this._layers;

    var i = 0;
    var j = 0;
    while (i < layers.length || j < sprites.length) {
        var which;
        if (i == layers.length) {
            which = 1;
        } else if (j == sprites.length) {
            which = 0;
        } else if (layers[i].pos_u > sprites[j].pos_u) {
            which = 1;
        } else if (layers[i].pos_v > sprites[j].pos_v) {
            which = 1;
        } else {
            which = 0;
        }

        if (which == 0) {
            var layer = layers[i];
            ++i;

            ctx.drawImage(this._pages[layer.page].canvas,
                    layer.src_x, layer.src_y, layer.src_w, layer.src_h,
                    layer.pos_x, layer.dst_y, layer.src_w, layer.src_h);
        } else {
            var sprite = sprites[j];
            ++j;

            sprite.draw(ctx);
        }
    }
};
