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

    var EXPECT_SIZES = 17;
    var alloc = ((1 + EXPECT_SIZES) * 4 + 7) & ~7;
    var base = asm['__adjust_stack'](alloc);

    asm['get_sizes'](base + 4, base);

    var view = new Int32Array(buffer, base, 1 + EXPECT_SIZES);
    console.assert(view[0] == EXPECT_SIZES,
            'expected sizes for ' + EXPECT_SIZES + ' types, but got ' + view[0]);

    var sizeof = {};
    var index = 0;
    var next = function() { return view[1 + index++]; };

    sizeof.ShapeChunk = next();
    sizeof.ShapeLayers = next();

    sizeof.BlockData = next();
    sizeof.BlockChunk = next();
    sizeof.LocalChunks = next();

    sizeof.Structure = next();

    sizeof.TerrainVertex = next();
    sizeof.TerrainGeomGen = next();

    sizeof.StructureTemplate = next();
    sizeof.ModelVertex = next();
    sizeof.StructureBuffer = next();
    sizeof.StructureBaseVertex = next();
    sizeof.StructureBaseGeomGen = next();
    sizeof.StructureAnimVertex = next();
    sizeof.StructureAnimGeomGen = next();

    sizeof.LightVertex = next();
    sizeof.LightGeomGen = next();

    console.assert(index == EXPECT_SIZES,
            'some items were left over after building sizeof', index, EXPECT_SIZES);

    return sizeof;
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

Asm.prototype._makeView = function(type, offset, bytes) {
    return new type(this.buffer, offset, bytes / type.BYTES_PER_ELEMENT);
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

/** @constructor */
function AsmGraphics(num_blocks, num_templates, num_model_vertexes,
        structures_size, geom_size) {
    var heap_end = HEAP_START;
    function alloc(size) {
        // 8-byte alignment
        heap_end = (heap_end + 7) & ~7;
        var pos = heap_end;
        heap_end += size;
        return pos;
    }

    this.num_blocks = num_blocks;
    this.num_templates = num_templates;
    this.num_model_vertexes = num_model_vertexes;

    this.block_data_bytes = num_blocks * SIZEOF.BlockData;
    this.template_data_bytes = num_templates * SIZEOF.StructureTemplate;
    this.model_vertex_bytes = num_model_vertexes * SIZEOF.ModelVertex;
    this.geom_buffer_bytes = geom_size;
    // TODO: sizeof(Structure) * num_structures
    this.structure_storage_bytes = structures_size

    this.LOCAL_CHUNKS = alloc(SIZEOF.LocalChunks);
    this.TERRAIN_GEOM_GEN = alloc(SIZEOF.TerrainGeomGen);
    this.STRUCTURE_BASE_GEOM_GEN = alloc(SIZEOF.StructureBaseGeomGen);
    this.STRUCTURE_ANIM_GEOM_GEN = alloc(SIZEOF.StructureAnimGeomGen);
    this.LIGHT_GEOM_GEN = alloc(SIZEOF.LightGeomGen);
    this.STRUCTURE_BUFFER = alloc(SIZEOF.StructureBuffer);

    this.BLOCK_DATA = alloc(this.block_data_bytes);
    this.TEMPLATE_DATA = alloc(this.template_data_bytes);
    this.MODEL_DATA = alloc(this.model_vertex_bytes);
    this.GEOM_BUFFER = alloc(this.geom_buffer_bytes);

    this.STRUCTURE_STORAGE = alloc(this.structure_storage_bytes);

    Asm.call(this, heap_end - HEAP_START);
}
AsmGraphics.prototype = Object.create(Asm.prototype);
exports.AsmGraphics = AsmGraphics;


AsmGraphics.prototype.blockDataView8 = function() {
    return this._makeView(Uint8Array, this.BLOCK_DATA, this.block_data_bytes);
};

AsmGraphics.prototype.blockDataView16 = function() {
    return this._makeView(Uint16Array, this.BLOCK_DATA, this.block_data_bytes);
};

AsmGraphics.prototype.chunkView = function(cx, cy) {
    var idx = (cy & (LOCAL_SIZE - 1)) * LOCAL_SIZE + (cx & (LOCAL_SIZE - 1));
    var offset = idx * SIZEOF.BlockChunk;
    return this._makeView(Uint16Array, this.LOCAL_CHUNKS + offset, SIZEOF.BlockChunk);
};

AsmGraphics.prototype.templateDataView8 = function() {
    return this._makeView(Uint8Array, this.TEMPLATE_DATA, this.template_data_bytes);
};

AsmGraphics.prototype.templateDataView16 = function() {
    return this._makeView(Uint16Array, this.TEMPLATE_DATA, this.template_data_bytes);
};

AsmGraphics.prototype.modelVertexView = function() {
    return this._makeView(Uint16Array, this.MODEL_DATA, this.model_vertex_bytes);
};


AsmGraphics.prototype.terrainGeomInit = function() {
    this._raw['terrain_geom_init'](
            this.TERRAIN_GEOM_GEN,
            this.BLOCK_DATA,
            this.block_data_bytes,
            this.LOCAL_CHUNKS);
};

AsmGraphics.prototype.terrainGeomReset = function(cx, cy) {
    this._raw['terrain_geom_reset'](this.TERRAIN_GEOM_GEN, cx, cy);
};

AsmGraphics.prototype.terrainGeomGenerate = function() {
    var output = this._stackAlloc(Int32Array, 2);

    this._raw['terrain_geom_generate'](
            this.TERRAIN_GEOM_GEN,
            this.GEOM_BUFFER,
            this.geom_buffer_bytes,
            output.byteOffset);

    var vertex_count = output[0];
    var more = (output[1] & 1) != 0;

    this._stackFree(output);

    return {
        geometry: this._makeView(Uint8Array, this.GEOM_BUFFER,
                          vertex_count * SIZEOF.TerrainVertex),
        more: more,
    };
};


AsmGraphics.prototype.structureBufferInit = function() {
    this._raw['structure_buffer_init'](
            this.STRUCTURE_BUFFER,
            this.STRUCTURE_STORAGE,
            this.structure_storage_bytes);
};

AsmGraphics.prototype.structureBufferInsert = function(id, x, y, z, template_id) {
    return this._raw['structure_buffer_insert'](
            this.STRUCTURE_BUFFER,
            id, x, y, z, template_id);
};

AsmGraphics.prototype.structureBufferRemove = function(idx) {
    return this._raw['structure_buffer_remove'](
            this.STRUCTURE_BUFFER,
            idx);
};

AsmGraphics.prototype.structureBufferSetOneshotStart = function(idx, oneshot_start) {
    this._raw['structure_buffer_set_oneshot_start'](
            this.STRUCTURE_BUFFER,
            idx, oneshot_start);
};


AsmGraphics.prototype.structureBaseGeomInit = function() {
    this._raw['structure_base_geom_init'](
            this.STRUCTURE_BASE_GEOM_GEN,
            this.STRUCTURE_BUFFER,
            this.TEMPLATE_DATA,
            this.template_data_bytes,
            this.MODEL_DATA,
            this.model_vertex_bytes);
};

AsmGraphics.prototype.structureBaseGeomReset = function(cx0, cy0, cx1, cy1, sheet) {
    this._raw['structure_base_geom_reset'](
            this.STRUCTURE_BASE_GEOM_GEN,
            cx0, cy0, cx1, cy1, sheet);
};

AsmGraphics.prototype.structureBaseGeomGenerate = function() {
    var output = this._stackAlloc(Int32Array, 2);

    this._raw['structure_base_geom_generate'](
            this.STRUCTURE_BASE_GEOM_GEN,
            this.GEOM_BUFFER,
            this.geom_buffer_bytes,
            output.byteOffset);

    var vertex_count = output[0];
    var more = (output[1] & 1) != 0;

    this._stackFree(output);

    return {
        geometry: this._makeView(Uint8Array, this.GEOM_BUFFER,
                          vertex_count * SIZEOF.StructureBaseVertex),
        more: more,
    };
};


AsmGraphics.prototype.structureAnimGeomInit = function() {
    this._raw['structure_anim_geom_init'](
            this.STRUCTURE_ANIM_GEOM_GEN,
            this.STRUCTURE_BUFFER,
            this.TEMPLATE_DATA,
            this.template_data_bytes);
};

AsmGraphics.prototype.structureAnimGeomReset = function(cx0, cy0, cx1, cy1, sheet) {
    this._raw['structure_anim_geom_reset'](
            this.STRUCTURE_ANIM_GEOM_GEN,
            cx0, cy0, cx1, cy1, sheet);
};

AsmGraphics.prototype.structureAnimGeomGenerate = function() {
    var output = this._stackAlloc(Int32Array, 2);

    this._raw['structure_anim_geom_generate'](
            this.STRUCTURE_ANIM_GEOM_GEN,
            this.GEOM_BUFFER,
            this.geom_buffer_bytes,
            output.byteOffset);

    var vertex_count = output[0];
    var more = (output[1] & 1) != 0;

    this._stackFree(output);

    return {
        geometry: this._makeView(Uint8Array, this.GEOM_BUFFER,
                          vertex_count * SIZEOF.StructureAnimVertex),
        more: more,
    };
};


AsmGraphics.prototype.lightGeomInit = function() {
    this._raw['light_geom_init'](
            this.LIGHT_GEOM_GEN,
            this.STRUCTURE_BUFFER,
            this.TEMPLATE_DATA,
            this.template_data_bytes);
};

AsmGraphics.prototype.lightGeomReset = function(cx0, cy0, cx1, cy1) {
    this._raw['light_geom_reset'](
            this.LIGHT_GEOM_GEN,
            cx0, cy0, cx1, cy1);
};

AsmGraphics.prototype.lightGeomGenerate = function() {
    var output = this._stackAlloc(Int32Array, 2);

    this._raw['light_geom_generate'](
            this.LIGHT_GEOM_GEN,
            this.GEOM_BUFFER,
            this.geom_buffer_bytes,
            output.byteOffset);

    var vertex_count = output[0];
    var more = (output[1] & 1) != 0;

    this._stackFree(output);

    return {
        geometry: this._makeView(Uint8Array, this.GEOM_BUFFER,
                          vertex_count * SIZEOF.LightVertex),
        more: more,
    };
};




// Test

Asm.prototype.test = function() {
};
