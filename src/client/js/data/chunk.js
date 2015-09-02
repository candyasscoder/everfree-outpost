var CHUNK_SIZE = 16;
var TILE_SIZE = 32;
var LOCAL_SIZE = 8;
exports.CHUNK_SIZE = CHUNK_SIZE;
exports.TILE_SIZE = TILE_SIZE;
exports.LOCAL_SIZE = LOCAL_SIZE;


/** @constructor */
function Chunk() {
    var count = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE;
    this._tiles = new Uint16Array(count);
}
exports.Chunk = Chunk;

Chunk.prototype.getId = function(x, y, z) {
    var idx = (z * CHUNK_SIZE + y) * CHUNK_SIZE + x;
    return this._tiles[idx];
};

Chunk.prototype.get = function(x, y, z) {
    return BlockDef.by_id[this.getId(x, y, z)];
};

Chunk.prototype.set = function(x, y, z, tile) {
    var tile_id;
    if (typeof tile === 'number') {
        tile_id = tile;
    } else if (typeof tile === 'object') {
        tile_id = tile.id;
    } else {
        console.assert(false, "Chunk.set: invalid tile", tile);
    }

    var idx = (z * CHUNK_SIZE + y) * CHUNK_SIZE + x;
    this._tiles[idx] = tile_id;
};


/** @constructor */
function BlockDef_(id, info) {
    this.id = id;
    this.shape = info['shape'];
    this.top = info['top'] || 0;
    this.bottom = info['bottom'] || 0;
    this.front = info['front'] || 0;
    this.back = info['back'] || 0;

    this.light_color = info['light_r'] == null ? [0, 0, 0] :
        [info['light_r'], info['light_g'], info['light_g']];
    this.light_radius = info['light_radius'] || 0;
}

// Closure compiler doesn't like having static items on functions.
var BlockDef = {};
exports.BlockDef = BlockDef;

BlockDef.by_id = [];

BlockDef.register = function(id, info) {
    if (info == null) {
        return;
    }

    var tile = new BlockDef_(id, info);
    while (BlockDef.by_id.length <= tile.id) {
        BlockDef.by_id.push(null);
    }
    BlockDef.by_id[tile.id] = tile;
};
