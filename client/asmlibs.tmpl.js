(function() {
    // asm.js module

    var module = (function(global, env, buffer) {
        'use asm';

        var HEAP8 = new global.Int8Array(buffer);
        var HEAP16 = new global.Int16Array(buffer);
        var HEAP32 = new global.Int32Array(buffer);
        var HEAPU8 = new global.Uint8Array(buffer);
        var HEAPU16 = new global.Uint16Array(buffer);
        var HEAPU32 = new global.Uint32Array(buffer);
        var HEAPF32 = new global.Float32Array(buffer);
        var HEAPF64 = new global.Float64Array(buffer);

        var STACKTOP = env.STACK_START|0;
        var STACK_MAX = env.STACK_END|0;

        var abort = env.abort;
        var _llvm_trap = env.abort;
        var _trace_ints = env.traceInts;
        var _phys_trace = env.physTrace;
        var _reset_phys_trace = env.resetPhysTrace;
        var _write_str = env.writeStr;
        var _flush_str = env.flushStr;
        var Math_imul = global.Math.imul;

        var tempRet0 = 0;

        function __adjust_stack(offset) {
            offset = offset|0;
            STACKTOP = STACKTOP + offset|0;
            if ((STACKTOP|0) >= (STACK_MAX|0)) abort();
            return (STACKTOP - offset)|0;
        }


        function _bitshift64Lshr(low, high, bits) {
            low = low|0; high = high|0; bits = bits|0;
            var ander = 0;
            if ((bits|0) < 32) {
                ander = ((1 << bits) - 1)|0;
                tempRet0 = high >>> bits;
                return (low >>> bits) | ((high&ander) << (32 - bits));
            }
            tempRet0 = 0;
            return (high >>> (bits - 32))|0;
        }

        function _bitshift64Shl(low, high, bits) {
            low = low|0; high = high|0; bits = bits|0;
            var ander = 0;
            if ((bits|0) < 32) {
                ander = ((1 << bits) - 1)|0;
                tempRet0 = (high << bits) | ((low&(ander << (32 - bits))) >>> (32 - bits));
                return low << bits;
            }
            tempRet0 = low << (bits - 32);
            return 0;
        }

        function _memset(ptr, value, num) {
            ptr = ptr|0; value = value|0; num = num|0;
            var stop = 0, value4 = 0, stop4 = 0, unaligned = 0;
            stop = (ptr + num)|0;
            if ((num|0) >= 20) {
                // This is unaligned, but quite large, so work hard to get to aligned settings
                value = value & 0xff;
                unaligned = ptr & 3;
                value4 = value | (value << 8) | (value << 16) | (value << 24);
                stop4 = stop & ~3;
                if (unaligned) {
                    unaligned = (ptr + 4 - unaligned)|0;
                    while ((ptr|0) < (unaligned|0)) { // no need to check for stop, since we have large num
                        HEAP8[((ptr)>>0)]=value;
                        ptr = (ptr+1)|0;
                    }
                }
                while ((ptr|0) < (stop4|0)) {
                    HEAP32[((ptr)>>2)]=value4;
                    ptr = (ptr+4)|0;
                }
            }
            while ((ptr|0) < (stop|0)) {
                HEAP8[((ptr)>>0)]=value;
                ptr = (ptr+1)|0;
            }
            return (ptr-num)|0;
        }

        // INSERT_EMSCRIPTEN_FUNCTIONS

        return ({
            __adjust_stack: __adjust_stack,
            collide: _collide,
            bake_chunk: _bake_chunk,
            test: _test,
        });
    });


    // Static data

    // Note: The `awk` script will break if INSERT_*_STATIC comes before
    // INSERT_*_FUNCTIONS.
    var static_data = new Uint8Array(
            // INSERT_EMSCRIPTEN_STATIC
            );


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
            abort: function() {
                console.assert(false, 'abort');
                throw 'abort';
            },

            traceInts: function(ptr, len) {
                var arr = [];
                var view = new Int32Array(buffer);
                for (var i = 0; i < len; ++i) {
                    arr.push(view[(ptr >> 2) + i]);
                }
                console.log(arr);
            },

            physTrace: function(x, y, w, h) {
                window.physTrace.push([x, y, w, h]);
            },

            resetPhysTrace: function() {
                window.physTrace = [];
            },

            writeStr: function(ptr, len) {
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

            flushStr: function() {
                console.log(msg_buffer);
                msg_buffer = '';
            },

            STACK_START: STACK_START,
            STACK_END: STACK_END,
        });
    };


    // Helper functions

    function memcpy(dest_buffer, dest_offset, src_buffer, src_offset, len) {
        var dest = new Int8Array(dest_buffer, dest_offset, len);
        var src = new Int8Array(src_buffer, src_offset, len);
        dest.set(src);
    }


    // window.Asm wrapper

    function Asm(heap_size) {
        // Buffer size must be a multiple of 4k.
        var min_size = HEAP_START + heap_size;
        var buffer_size = (min_size + 0x0fff) & ~0x0fff;
        this.buffer = new ArrayBuffer(buffer_size);

        this.memcpy(STATIC_START, static_data);
        this._raw = module(window, module_env(this.buffer), this.buffer);
    }

    window.Asm = Asm;

    Asm.prototype = {
        '_stackAlloc': function(type, count) {
            var size = count * type.BYTES_PER_ELEMENT;
            var base = this._raw.__adjust_stack((size + 7) & ~7);
            return new type(this.buffer, base, count);
        },

        '_stackFree': function(view) {
            var size = view.byteLength;
            this._raw.__adjust_stack(-((size + 7) & ~7));
        },

        '_storeVec': function(view, offset, v) {
            view[offset + 0] = v.x;
            view[offset + 1] = v.y;
            view[offset + 2] = v.z;
        },

        'memcpy': function(dest_offset, data) {
            memcpy(this.buffer, dest_offset, data.buffer, data.byteOffset, data.byteLength);
        },

        'collide': function(pos, size, velocity) {
            var input = this._stackAlloc(Int32Array, 9);
            var output = this._stackAlloc(Int32Array, 4);

            this._storeVec(input, 0, pos);
            this._storeVec(input, 3, size);
            this._storeVec(input, 6, velocity);

            this._raw.collide(input.byteOffset, output.byteOffset);

            var result = ({
                x: output[0],
                y: output[1],
                z: output[2],
                t: output[3],
            });

            this._stackFree(input);
            this._stackFree(output);

            return result;
        },

        'chunkShapeView': function(idx) {
            var size = 16 * 16 * 16;
            return new Uint8Array(this.buffer, HEAP_START + idx * size, size);
        },

        'bakeChunk': function(flags) {
            this.memcpy(HEAP_START, flags);
            var counts = this._stackAlloc(Int32Array, 2);

            var layers_start = HEAP_START + ((flags.byteLength + 0x0fff) & ~0x0fff);
            this._raw.bake_chunk(HEAP_START, layers_start, counts.byteOffset);

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
                    'min': new Vec(view8[base8 + 0], view8[base8 + 1], view8[base8 + 2]),
                    'max': new Vec(view8[base8 + 3], view8[base8 + 4], view8[base8 + 5]),
                    'pos_x': pos & 0xf,
                    'pos_y': (pos >> 4) & 0x1f,
                    'page': pos >> 9,
                });
            }

            return { 'layers': layers, 'pages': page_count };
        },

        'test': function(pos, size, velocity) {
            var input = this._stackAlloc(Int32Array, 9);
            var output = this._stackAlloc(Int32Array, 4);

            this._storeVec(input, 0, pos);
            this._storeVec(input, 3, size);
            this._storeVec(input, 6, velocity);

            this._raw.test(input.byteOffset, output.byteOffset);

            var result = ({
                x: output[0],
                y: output[1],
                z: output[2],
                t: output[3],
            });

            this._stackFree(input);
            this._stackFree(output);

            return result;
        },

    };
})();
