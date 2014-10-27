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
    return TileDef.by_id[this.getId(x, y, z)];
};

Chunk.prototype.set = function(x, y, z, tile) {
    var tile_id;
    if (typeof tile === 'number') {
        tile_id = tile;
    } else if (typeof tile === 'string') {
        tile_id = TileDef.by_name[tile].id;
    } else if (typeof tile === 'object') {
        tile_id = tile.id;
    } else {
        console.assert(false, "Chunk.set: invalid tile", tile);
    }

    var idx = (z * CHUNK_SIZE + y) * CHUNK_SIZE + x;
    this._tiles[idx] = tile_id;
};


/** @constructor */
function TileDef_(id, info) {
    this.id = id;
    this.name = info['name'];
    this.shape = info['shape'];

    var front = info['front'];
    if (front != null) {
        this.front = front[1] * 16 + front[0];
    } else {
        this.front = 0;
    }

    var back = info['back'];
    if (back != null) {
        this.back = back[1] * 16 + back[0];
    } else {
        this.back = 0;
    }

    var top = info['top'];
    if (top != null) {
        this.top = top[1] * 16 + top[0];
    } else {
        this.top = 0;
    }

    var bottom = info['bottom'];
    if (bottom != null) {
        this.bottom = bottom[1] * 16 + bottom[0];
    } else {
        this.bottom = 0;
    }
}

// Closure compiler doesn't like having static items on functions.
var TileDef = {};
exports.TileDef = TileDef;

TileDef.by_id = [];
TileDef.by_name = {};

TileDef.register = function(id, info) {
    if (info == null) {
        return;
    }

    var tile = new TileDef_(id, info);
    while (TileDef.by_id.length <= tile.id) {
        TileDef.by_id.push(null);
    }
    TileDef.by_id[tile.id] = tile;
    TileDef.by_name[tile.name] = tile;
};
