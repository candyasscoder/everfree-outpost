var module = window['asmlibs_code'];
var static_data = window['asmlibs_data'];

var Vec = require('util/vec').Vec;
var decodeUtf8 = require('util/misc').decodeUtf8;
var LOCAL_SIZE = require('data/chunk').LOCAL_SIZE;


// Memory layout

// Emscripten puts the first static at address 8 to  avoid storing data at
// address 0.
var STATIC_START = 8;  
var STATIC_SIZE = static_data.byteLength;
var STATIC_END = STATIC_START + STATIC_SIZE;

// Align STACK_START to an 8-byte boundary.
var STACK_START = (STATIC_END + 7) & ~7;
// Give at least 4k for stack, and align to 4k boundary.
var STACK_END = (STACK_START + 0x1000 + 0x0fff) & ~0x0fff;
var STACK_SIZE = STACK_END - STACK_START;
console.assert(STACK_SIZE >= 0x1000, 'want at least 4kb for stack');

var HEAP_START = STACK_END;


// External functions

var msg_buffer = '';

var module_env = function(buffer, callback_array) {
    return ({
        'abort': function() {
            console.assert(false, 'abort');
            throw 'abort';
        },

        'traceInts': function(ptr, len) {
            var arr = [];
            var view = new Int32Array(buffer);
            for (var i = 0; i < len; ++i) {
                arr.push(view[(ptr >> 2) + i]);
            }
            console.log(arr);
        },

        'physTrace': function(x, y, w, h) {
            window.physTrace.push([x, y, w, h]);
        },

        'resetPhysTrace': function() {
            window.physTrace = [];
        },

        'writeStr': function(ptr, len) {
            var view = new Uint8Array(buffer, ptr, len);
            msg_buffer += decodeUtf8(view);
        },

        'flushStr': function() {
            console.log(msg_buffer);
            msg_buffer = '';
        },

        'runCallback': function(index, arg_base, arg_len) {
            var cb = callback_array[index];
            console.assert(cb != null, 'bad callback index: ', index);

            var args = new Int32Array(buffer, arg_base, arg_len);
            return cb.apply(window, args);
        },

        'STACK_START': STACK_START,
        'STACK_END': STACK_END,
    });
};


// Helper functions

function memcpy(dest_buffer, dest_offset, src_buffer, src_offset, len) {
    var dest = new Int8Array(dest_buffer, dest_offset, len);
    var src = new Int8Array(src_buffer, src_offset, len);
    dest.set(src);
}


// Buffer size computations
var SIZEOF = (function() {
    var buffer = new ArrayBuffer((HEAP_START + 0xffff) & ~0xffff);
    memcpy(buffer, STATIC_START,
            static_data.buffer, static_data.byteOffset, static_data.byteLength);
    var asm = module(window, module_env(buffer), buffer);

    var EXPECT_SIZES = 16;
    var alloc = ((1 + EXPECT_SIZES) * 4 + 7) & ~7;
    var base = asm['__adjust_stack'](alloc);

    asm['get_sizes'](base + 4, base);

    var view = new Int32Array(buffer, base, 1 + EXPECT_SIZES);
    console.assert(view[0] == EXPECT_SIZES,
            'expected sizes for ' + EXPECT_SIZES + ' types, but got ' + view[0]);

    return ({
        ShapeChunk: view[1],
        ShapeLayers: view[2],

        BlockDisplay: view[3],
        BlockData: view[4],
        BlockChunk: view[5],
        LocalChunks: view[6],

        TerrainVertex: view[7],
        TerrainGeometryBuffer: view[8],

        StructureTemplate: view[9],
        StructureTemplateData: view[10],
        StructureBuffer: view[11],
        StructureVertex: view[12],
        StructureGeometryBuffer: view[13],

        LightGeometryState: view[14],
        LightVertex: view[15],
        LightGeometryBuffer: view[16],
    });
})();
exports.SIZEOF = SIZEOF;


// window.Asm wrapper

function next_heap_size(size) {
    // "the heap object's byteLength must be either 2^n for n in [12, 24) or
    // 2^24 * n for n ≥ 1"
    if (size <= (1 << 12)) {
        return (1 << 12);
    } else if (size >= (1 << 24)) {
        return (size | ((1 << 24) - 1)) + 1;
    } else {
        for (var i = 12 + 1; i < 24; ++i) {
            if (size <= (1 << i)) {
                return (1 << i);
            }
        }
        console.assert(false, 'failed to compute next heap size for', size);
        return (1 << 24);
    }
}

/** @constructor */
function Asm(heap_size) {
    // Buffer size must be a multiple of 4k.
    var min_size = HEAP_START + heap_size;
    this.buffer = new ArrayBuffer(next_heap_size(min_size));

    this._callbacks = [];

    this.memcpy(STATIC_START, static_data);
    this._raw = module(window, module_env(this.buffer, this._callbacks), this.buffer);
}
exports.Asm = Asm;

Asm.prototype._stackAlloc = function(type, count) {
    var size = count * type.BYTES_PER_ELEMENT;
    var base = this._raw['__adjust_stack']((size + 7) & ~7);
    return new type(this.buffer, base, count);
};

Asm.prototype._stackFree = function(view) {
    var size = view.byteLength;
    this._raw['__adjust_stack'](-((size + 7) & ~7));
};

Asm.prototype._callbackAlloc = function(cb) {
    this._callbacks.push(cb);
    return this._callbacks.length - 1;
};

Asm.prototype._callbackFree = function(idx) {
    console.assert(idx == this._callbacks.length - 1);
    this._callbacks.pop();
};

Asm.prototype._storeVec = function(view, offset, v) {
    view[offset + 0] = v.x;
    view[offset + 1] = v.y;
    view[offset + 2] = v.z;
};

Asm.prototype.memcpy = function(dest_offset, data) {
    memcpy(this.buffer, dest_offset, data.buffer, data.byteOffset, data.byteLength);
};


// Physics

var PHYSICS_HEAP_START = HEAP_START;

var SHAPE_LAYERS_START = HEAP_START;
var SHAPE_LAYERS_END = SHAPE_LAYERS_START + SIZEOF.ShapeLayers * LOCAL_SIZE * LOCAL_SIZE;

var PHYSICS_HEAP_END = SHAPE_LAYERS_END;

Asm.prototype.collide = function(pos, size, velocity) {
    var input = this._stackAlloc(Int32Array, 9);
    var output = this._stackAlloc(Int32Array, 4);

    this._storeVec(input, 0, pos);
    this._storeVec(input, 3, size);
    this._storeVec(input, 6, velocity);

    this._raw['collide'](SHAPE_LAYERS_START, input.byteOffset, output.byteOffset);

    var result = ({
        x: output[0],
        y: output[1],
        z: output[2],
        t: output[3],
    });

    this._stackFree(output);
    this._stackFree(input);

    return result;
};

Asm.prototype.setRegionShape = function(pos, size, layer, shape) {
    var input_bounds = this._stackAlloc(Int32Array, 6);
    var input_shape = this._stackAlloc(Uint8Array, shape.length);

    this._storeVec(input_bounds, 0, pos);
    this._storeVec(input_bounds, 3, pos.add(size));
    input_shape.set(shape);

    this._raw['set_region_shape'](SHAPE_LAYERS_START,
            input_bounds.byteOffset, layer,
            input_shape.byteOffset, input_shape.length);
    this._raw['refresh_shape_cache'](SHAPE_LAYERS_START, input_bounds.byteOffset);

    this._stackFree(input_shape);
    this._stackFree(input_bounds);
};

Asm.prototype.clearRegionShape = function(pos, size, layer) {
    var volume = size.x * size.y * size.z;

    var input_bounds = this._stackAlloc(Int32Array, 6);
    var input_shape = this._stackAlloc(Uint8Array, volume);

    this._storeVec(input_bounds, 0, pos);
    this._storeVec(input_bounds, 3, pos.add(size));
    for (var i = 0; i < volume; ++i) {
        input_shape[i] = 0;
    }

    this._raw['set_region_shape'](PHYSICS_HEAP_START,
            input_bounds.byteOffset, layer,
            input_shape.byteOffset, input_shape.length);
    this._raw['refresh_shape_cache'](PHYSICS_HEAP_START, input_bounds.byteOffset);

    this._stackFree(input_shape);
    this._stackFree(input_bounds);
};

Asm.prototype.refreshShapeLayers = function(pos, size) {
    var input = this._stackAlloc(Int32Array, 6);

    this._storeVec(input, 0, pos);
    this._storeVec(input, 3, pos.add(size));

    this._raw['refresh_shape_cache'](SHAPE_LAYERS_START, input.byteOffset);

    this._stackFree(input);
};

Asm.prototype.shapeLayerView = function(chunk_idx, layer) {
    var chunk_offset = chunk_idx * SIZEOF.ShapeLayers;
    var layer_offset = (1 + layer) * SIZEOF.ShapeChunk;

    return new Uint8Array(this.buffer,
            SHAPE_LAYERS_START + chunk_offset + layer_offset, SIZEOF.ShapeChunk);
};

Asm.prototype.findCeiling = function(pos) {
    var input = this._stackAlloc(Int32Array, 3);
    this._storeVec(input, 0, pos);

    var result = this._raw['find_ceiling'](SHAPE_LAYERS_START, input.byteOffset);

    this._stackFree(input);
    return result;
};

exports.getPhysicsHeapSize = function() {
    return PHYSICS_HEAP_END - PHYSICS_HEAP_START;
};


// Graphics

var GRAPHICS_HEAP_START = HEAP_START;

var BLOCK_DATA_START = HEAP_START;
var BLOCK_DATA_END = BLOCK_DATA_START + SIZEOF.BlockData;

var LOCAL_CHUNKS_START = BLOCK_DATA_END;
var LOCAL_CHUNKS_END = LOCAL_CHUNKS_START + SIZEOF.LocalChunks;

var CHUNK_START = LOCAL_CHUNKS_END;
var CHUNK_END = CHUNK_START + SIZEOF.BlockChunk;

var GEOM_START = CHUNK_END;
var GEOM_END = GEOM_START + SIZEOF.TerrainGeometryBuffer;

var TEMPLATE_DATA_START = GEOM_END;
var TEMPLATE_DATA_END = TEMPLATE_DATA_START + SIZEOF.StructureTemplateData;

var STRUCTURES_START = TEMPLATE_DATA_END;
var STRUCTURES_END = STRUCTURES_START + SIZEOF.StructureBuffer;

var STRUCTURE_GEOM_START = STRUCTURES_END;
var STRUCTURE_GEOM_END = STRUCTURE_GEOM_START + SIZEOF.StructureGeometryBuffer;

var LIGHT_STATE_START = STRUCTURE_GEOM_END;
var LIGHT_STATE_END = LIGHT_STATE_START + SIZEOF.LightGeometryState;

var LIGHT_GEOM_START = LIGHT_STATE_END;
var LIGHT_GEOM_END = LIGHT_GEOM_START + SIZEOF.LightGeometryBuffer;

var GRAPHICS_HEAP_END = LIGHT_GEOM_END;

exports.getGraphicsHeapSize = function() {
    return GRAPHICS_HEAP_END - GRAPHICS_HEAP_START;
};


Asm.prototype.blockDataView8 = function() {
    return new Uint8Array(this.buffer, BLOCK_DATA_START, SIZEOF.BlockData >> 0);
};

Asm.prototype.blockDataView16 = function() {
    return new Uint16Array(this.buffer, BLOCK_DATA_START, SIZEOF.BlockData >> 1);
};

Asm.prototype.chunkView = function() {
    return new Uint16Array(this.buffer, CHUNK_START, SIZEOF.BlockChunk >> 1);
};

Asm.prototype.loadChunk = function(cx, cy) {
    this._raw['load_chunk'](LOCAL_CHUNKS_START, CHUNK_START, cx, cy);
};

Asm.prototype.generateTerrainGeometry = function(cx, cy, max_z) {
    if (max_z != 16) {
        var len = this._raw['generate_sliced_terrain_geometry'](
                LOCAL_CHUNKS_START, BLOCK_DATA_START, GEOM_START, cx, cy, max_z);
    } else {
        var len = this._raw['generate_terrain_geometry'](
                LOCAL_CHUNKS_START, BLOCK_DATA_START, GEOM_START, cx, cy);
    }
    return new Uint8Array(this.buffer, GEOM_START, SIZEOF.TerrainVertex * len);
};

Asm.prototype._localChunksView = function() {
    return new Uint16Array(this.buffer, LOCAL_CHUNKS_START, SIZEOF.LocalChunks >> 1);
};

Asm.prototype._geometryView = function() {
    return new Uint16Array(this.buffer, GEOM_START, SIZEOF.TerrainGeometryBuffer >> 1);
};


Asm.prototype.templateDataView8 = function() {
    return new Uint8Array(this.buffer, TEMPLATE_DATA_START, SIZEOF.StructureTemplateData);
};

Asm.prototype.templateDataView16 = function() {
    return new Uint16Array(this.buffer, TEMPLATE_DATA_START, SIZEOF.StructureTemplateData >> 1);
};

Asm.prototype.initStructureBuffer = function() {
    this._raw['init_structure_buffer'](STRUCTURES_START, TEMPLATE_DATA_START);
};

Asm.prototype.addStructure = function(x, y, z, template_id) {
    return this._raw['add_structure'](STRUCTURES_START, x, y, z, template_id);
};

Asm.prototype.removeStructure = function(idx) {
    this._raw['remove_structure'](STRUCTURES_START, idx);
};

Asm.prototype.resetStructureGeometry = function() {
    this._raw['reset_structure_geometry'](STRUCTURES_START);
};

Asm.prototype.generateStructureGeometry = function(cx, cy, max_z) {
    var output = this._stackAlloc(Int32Array, 2);

    this._raw['generate_structure_geometry'](
            STRUCTURES_START, STRUCTURE_GEOM_START, cx, cy, max_z, output.byteOffset);

    var output8 = new Uint8Array(output.buffer, output.byteOffset, output.byteLength);
    var vertex_count = output[0];
    var result = {
        geometry: new Uint8Array(this.buffer, STRUCTURE_GEOM_START,
                          SIZEOF.StructureVertex * vertex_count),
        sheet: output8[4],
        more: output8[5],
    };

    this._stackFree(output);
    return result;
};

Asm.prototype.generateStructureAnimGeometry = function(cx, cy, max_z) {
    var output = this._stackAlloc(Int32Array, 2);

    this._raw['generate_structure_anim_geometry'](
            STRUCTURES_START, STRUCTURE_GEOM_START, cx, cy, max_z, output.byteOffset);

    var output8 = new Uint8Array(output.buffer, output.byteOffset, output.byteLength);
    var vertex_count = output[0];
    var result = {
        geometry: new Uint8Array(this.buffer, STRUCTURE_GEOM_START,
                          SIZEOF.StructureVertex * vertex_count),
        sheet: output8[4],
        more: output8[5],
    };

    this._stackFree(output);
    return result;
};


Asm.prototype.initLightState = function() {
    this._raw['init_light_state'](LIGHT_STATE_START, BLOCK_DATA_START, TEMPLATE_DATA_START);
};

Asm.prototype.resetLightGeometry = function(cx0, cy0, cx1, cy1) {
    this._raw['reset_light_geometry'](LIGHT_STATE_START, cx0, cy0, cx1, cy1);
}

Asm.prototype.generateLightGeometry = function() {
    var output = this._stackAlloc(Int32Array, 2);

    this._raw['generate_light_geometry'](
            LIGHT_STATE_START,
            LIGHT_GEOM_START,
            LOCAL_CHUNKS_START,
            STRUCTURES_START,
            output.byteOffset);

    var vertex_count = output[0];
    var more = (output[1] & 1) != 0;

    var result = {
        geometry: new Uint8Array(this.buffer, LIGHT_GEOM_START,
                          SIZEOF.LightVertex * vertex_count),
        more: more,
    };

    this._stackFree(output);
    return result;
};


// Test

Asm.prototype.test = function() {
};
