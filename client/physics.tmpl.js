(function() {
    // Memory layout

    var STACK_START = 8;    // Must avoid storing data at address 0
    var STACK_END = 3 * 1024;
    var STACK_SIZE = STACK_END - STACK_START;

    var OUTPUT_START = STACK_END;
    var OUTPUT_SIZE = 512;
    var OUTPUT_END = OUTPUT_START + OUTPUT_SIZE;

    var INPUT_START = OUTPUT_END;
    var INPUT_SIZE = 512;
    var INPUT_END = INPUT_START + INPUT_SIZE;

    var HEAP_START = INPUT_END;


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
        var _phys_trace = env.physTrace;
        var Math_imul = global.Math.imul;

        // INSERT_EMSCRIPTEN_FUNCTIONS

        return ({
            collide: _collide,
        });
    });

    var module_env = {
        abort: function() {
            console.assert(false, 'abort');
            throw 'abort';
        },
        physTrace: function(x, y, z) {
            window.physTrace.push([x,y,z]);
        },
        STACK_START: STACK_START,
        STACK_END: STACK_END,
    };

    var init_module = (function(buffer) {
        return module(window, module_env, buffer);
    });


    // Helper functions

    function memcpy(dest_buffer, dest_offset, src_buffer, src_offset, len) {
        console.log(dest_buffer, dest_offset, len);
        var dest = new Int8Array(dest_buffer, dest_offset, len);
        var src = new Int8Array(src_buffer, src_offset, len);
        dest.set(src);
    }

    function make_region(type, buffer, offset, byte_len) {
        var elem_bytes = type.BYTES_PER_ELEMENT;
        console.assert(byte_len % elem_bytes == 0,
                'make_region: byte_len must be a multiple of BYTES_PER_ELEMENT');
        var len = (byte_len / elem_bytes)|0;
        return new type(buffer, offset, len);
    }


    // Main ChunkPhysicsAsm implementation

    function ChunkPhysicsAsm(shapes) {
        // Buffer size must be a multiple of 8 (for HEAPF64).
        var heap_size = (shapes.byteLength + 7) & ~7;
        this.buffer = new ArrayBuffer(HEAP_START + heap_size);

        this.asm = init_module(this.buffer);

        this.input = make_region(Int32Array, this.buffer, INPUT_START, INPUT_SIZE);
        this.output = make_region(Int32Array, this.buffer, OUTPUT_START, OUTPUT_SIZE);
        memcpy(this.buffer, HEAP_START, shapes.buffer, shapes.byteOffset, shapes.byteLength);
    }

    ChunkPhysicsAsm.prototype = {
        'collide': function(pos, size, velocity) {
            this.input[0] = pos.x;
            this.input[1] = pos.y;
            this.input[2] = pos.z;
            this.input[3] = size.x;
            this.input[4] = size.y;
            this.input[5] = size.z;
            this.input[6] = velocity.x;
            this.input[7] = velocity.y;
            this.input[8] = velocity.z;

            this.asm.collide(INPUT_START, OUTPUT_START);

            var result = ({
                x: this.output[0],
                y: this.output[1],
                z: this.output[2],
                t: this.output[3],
                d: this.output[4],
                type: this.output[5],
            });
            //console.log(result);
            return result;
        },
    };

    window.ChunkPhysicsAsm = ChunkPhysicsAsm;
})();
