var module = window['asmlibs_code'];
var static_data = window['asmlibs_data'];

var Vec = require('vec').Vec;


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

var module_env = function(buffer, callback_handler) {
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
            var utf8_buffer = '';
            var saw_utf8 = false;
            var view = new Uint8Array(buffer, ptr, len);
            for (var i = 0; i < len; ++i) {
                var byte_ = view[i];
                utf8_buffer += String.fromCharCode(byte_);
                if (byte_ >= 0x80) {
                    saw_utf8 = true;
                }
            }

            if (saw_utf8) {
                utf8_buffer = decodeURIComponent(escape(utf8_buffer));
            }
            msg_buffer += utf8_buffer;
        },

        'flushStr': function() {
            console.log(msg_buffer);
            msg_buffer = '';
        },

        'runCallback': callback_handler || (function() {
            console.assert(false, 'tried to use callback, but there was no callback handler');
            throw 'no callback handler';
        }),

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


// window.RawAsm wrapper

/** @constructor */
function Asm(heap_size, callback_handler) {
    // Buffer size must be a multiple of 4k.
    var min_size = HEAP_START + heap_size;
    // TODO: properly implement the actual asm.js heap size rules
    var buffer_size = (min_size + 0x0ffff) & ~0x0ffff;
    this.buffer = new ArrayBuffer(buffer_size);

    this.memcpy(STATIC_START, static_data);
    this._raw = module(window, module_env(this.buffer, callback_handler), this.buffer);
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


var BLOCKS_START = HEAP_START;
var BLOCKS_LEN = 1024 * 4;
var BLOCKS_SIZE = BLOCKS_LEN * 2;
var BLOCKS_END = BLOCKS_START + BLOCKS_SIZE;

var CHUNK_START = BLOCKS_END;
var CHUNK_LEN = 16 * 16 * 16;
var CHUNK_SIZE = CHUNK_LEN * 2;
var CHUNK_END = CHUNK_START + CHUNK_SIZE;

var XV_START = CHUNK_END;
var XV_LEN = 8 * 8 * 16 * 16 * 16 * 4;
var XV_SIZE = XV_LEN * 2;
var XV_END = XV_START + XV_SIZE;

var SPRITES_START = XV_END;
var SPRITES_LEN = 8 * 1024;
var SPRITES_SIZE = SPRITES_LEN * 2;
var SPRITES_END = SPRITES_START + SPRITES_SIZE;

Asm.prototype.render = function(x, y, w, h, sprites) {
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

    this._raw['render'](XV_START, x, y, w, h, SPRITES_START, sprites.length);
};

Asm.prototype.updateXvData = function(i, j) {
    this._raw['update_xv_data'](XV_START, BLOCKS_START, CHUNK_START, i, j);
};

Asm.getRendererHeapSize = function() {
    return BLOCKS_SIZE + CHUNK_SIZE + XV_SIZE + SPRITES_SIZE;
};

Asm.prototype.xvDataView = function() {
    return new Uint16Array(this.buffer, XV_START, XV_LEN);
};

Asm.prototype.blockDataView = function() {
    return new Uint16Array(this.buffer, BLOCKS_START, BLOCKS_LEN);
};

Asm.prototype.chunkDataView = function() {
    return new Uint16Array(this.buffer, CHUNK_START, CHUNK_LEN);
};

Asm.prototype.spritesView = function() {
    return new Uint16Array(this.buffer, SPRITES_START, SPRITES_LEN);
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