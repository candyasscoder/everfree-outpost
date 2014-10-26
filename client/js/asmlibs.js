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

var module_env = function(buffer) {
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
function Asm(heap_size) {
    // Buffer size must be a multiple of 4k.
    var min_size = HEAP_START + heap_size;
    var buffer_size = (min_size + 0x0fff) & ~0x0fff;
    this.buffer = new ArrayBuffer(buffer_size);

    this.memcpy(STATIC_START, static_data);
    this._raw = module(window, module_env(this.buffer), this.buffer);
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

    this._stackFree(input);
    this._stackFree(output);

    return result;
};

Asm.prototype.chunkShapeView = function(idx) {
    var size = 16 * 16 * 16;
    return new Uint8Array(this.buffer, HEAP_START + idx * size, size);
};

Asm.prototype.bakeChunk = function() {
    var counts = this._stackAlloc(Int32Array, 2);

    var flags_size = 16 * 16 * 16;
    var layers_start = HEAP_START + flags_size;
    this._raw['bake_chunk'](HEAP_START, layers_start, counts.byteOffset);

    var layer_count = counts[0];
    var page_count = counts[1];
    this._stackFree(counts);

    var view8 = new Uint8Array(this.buffer, layers_start);
    var view16 = new Uint16Array(this.buffer, layers_start);
    var layers = [];
    for (var i = 0; i < layer_count; ++i) {
        var base8 = i * 8;
        var pos = view16[i * 4 + 3];
        layers.push({
            min: new Vec(view8[base8 + 0], view8[base8 + 1], view8[base8 + 2]),
            max: new Vec(view8[base8 + 3], view8[base8 + 4], view8[base8 + 5]),
            pos_x: pos & 0xf,
            pos_y: (pos >> 4) & 0x1f,
            page: pos >> 9,
        });
    }

    return { layers: layers, pages: page_count };
};

Asm.prototype.chunkFlagsView = function() {
    var size = 16 * 16 * 16;
    return new Uint8Array(this.buffer, HEAP_START, size);
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

    this._stackFree(input);
    this._stackFree(output);

    return result;
};
