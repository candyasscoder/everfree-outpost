var module = window['asmlibs_code'];
var static_data = window['asmlibs_data'];

var Vec = require('vec').Vec;
var decodeUtf8 = require('util').decodeUtf8;


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

    var EXPECT_SIZES = 5;
    var alloc = ((1 + EXPECT_SIZES) * 4 + 7) & ~7;
    var base = asm['__adjust_stack'](alloc);

    asm['get_sizes'](base + 4, base);

    var view = new Int32Array(buffer, base, 1 + EXPECT_SIZES);
    console.assert(view[0] == EXPECT_SIZES,
            'expected sizes for ' + EXPECT_SIZES + ' types, but got ' + view[0]);

    console.log(view);

    return ({
        XvData: view[1],
        Sprite: view[2],
        BlockData: view[3],
        ChunkData: view[4],
        GeometryBuffer: view[5],
    });
})();


// window.Asm wrapper

/** @constructor */
function Asm(heap_size) {
    // Buffer size must be a multiple of 4k.
    var min_size = HEAP_START + heap_size;
    // TODO: properly implement the actual asm.js heap size rules
    var buffer_size = (min_size + 0x0ffff) & ~0x0ffff;
    this.buffer = new ArrayBuffer(buffer_size);

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


Asm.prototype.collide = function(pos, size, velocity) {
    var input = this._stackAlloc(Int32Array, 9);
    var output = this._stackAlloc(Int32Array, 4);

    this._storeVec(input, 0, pos);
    this._storeVec(input, 3, size);
    this._storeVec(input, 6, velocity);

    this._raw['collide'](input.byteOffset, output.byteOffset);

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

Asm.prototype.chunkShapeView = function(idx) {
    var size = 16 * 16 * 16;
    return new Uint8Array(this.buffer, HEAP_START + idx * size, size);
};


var RENDER_HEAP_START = HEAP_START;

var BLOCKS_START = HEAP_START;
var BLOCKS_END = BLOCKS_START + SIZEOF.BlockData;

var CHUNK_START = BLOCKS_END;
var CHUNK_END = CHUNK_START + SIZEOF.ChunkData;

var SPRITES_START = CHUNK_END;
var SPRITES_END = SPRITES_START + SIZEOF.Sprite * 1024;

var GEOM_START = SPRITES_END;
var GEOM_END = GEOM_START + SIZEOF.GeometryBuffer;

var XV_START = GEOM_END;
var XV_END = XV_START + SIZEOF.XvData;

var RENDER_HEAP_END = XV_END;

Asm.prototype.render = function(x, y, w, h, sprites, draw_terrain, draw_sprite) {
    var draw_terrain_idx = this._callbackAlloc(draw_terrain);
    var draw_sprite_idx = this._callbackAlloc(draw_sprite);

    var view = this.spritesView();
    for (var i = 0; i < sprites.length; ++i) {
        var base = i * 8;
        var sprite = sprites[i];
        view[base + 0] = i;
        view[base + 1] = sprite.ref_x;
        view[base + 2] = sprite.ref_y;
        view[base + 3] = sprite.ref_z;
        view[base + 4] = sprite.width;
        view[base + 5] = sprite.height;
        view[base + 6] = sprite.anchor_x;
        view[base + 7] = sprite.anchor_y;
    }

    this._raw['render'](XV_START, x, y, w, h, SPRITES_START, sprites.length,
            draw_terrain_idx, draw_sprite_idx);

    this._callbackFree(draw_sprite_idx);
    this._callbackFree(draw_terrain_idx);
};

Asm.prototype.updateXvData = function(i, j) {
    this._raw['update_xv_data'](XV_START, BLOCKS_START, CHUNK_START, i, j);
};

Asm.prototype.generateGeometry = function(i, j) {
    var count = this._stackAlloc(Int32Array, 1);
    this._raw['generate_geometry'](XV_START, GEOM_START, i, j, count.byteOffset);
    var result = this.geomView().subarray(0, 4 * count[0]);
    this._stackFree(count);
    return result;
};

Asm.getRendererHeapSize = function() {
    return RENDER_HEAP_END - RENDER_HEAP_START;
};

Asm.prototype.blockDataView = function() {
    return new Uint16Array(this.buffer, BLOCKS_START, SIZEOF.BlockData >> 1);
};

Asm.prototype.chunkDataView = function() {
    return new Uint16Array(this.buffer, CHUNK_START, SIZEOF.ChunkData >> 1);
};

Asm.prototype.spritesView = function() {
    return new Uint16Array(this.buffer, SPRITES_START, (SIZEOF.Sprite * 1024) >> 1);
};

Asm.prototype.geomView = function() {
    return new Uint8Array(this.buffer, GEOM_START, SIZEOF.GeometryBuffer);
};


Asm.prototype.test = function(pos, size, velocity) {
    var input = this._stackAlloc(Int32Array, 9);
    var output = this._stackAlloc(Int32Array, 4);

    this._storeVec(input, 0, pos);
    this._storeVec(input, 3, size);
    this._storeVec(input, 6, velocity);

    this._raw['test'](input.byteOffset, output.byteOffset);

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
