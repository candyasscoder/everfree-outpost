(function() {
    var buffer = new ArrayBuffer(8192);

    var STACK_SIZE = 3072;
    var IO_SIZE = 512;
    var INPUT_ADDR = STACK_SIZE;
    var OUTPUT_ADDR = STACK_SIZE + IO_SIZE;
    var PREFIX_SIZE = STACK_SIZE + 2 * IO_SIZE;

    function abort() {
        console.assert(false, 'abort');
        throw 'abort';
    }

    function physTrace(x, y, z) {
        window.physTrace.push([x,y,z]);
    }

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

        var STACKTOP = 0;
        var STACK_MAX = 3072;

        var abort = env.abort;
        var _llvm_trap = env.abort;
        var _phys_trace = env.physTrace;
        var Math_imul = global.Math.imul;

        // INSERT_EMSCRIPTEN_FUNCTIONS

        return ({
            collide: _collide,
        });
    });


    function ChunkPhysicsAsm(shapes) {
        var byteLen = shapes.byteLength;
        console.assert(byteLen % 4 == 0,
                'ChunkPhysicsAsm: shape buffer must be a multiple of 4 bytes');
        var wordLen = (byteLen / 4)|0;

        // Buffer size must be a multiple of 8.
        var bufferLen = (byteLen + 7) & ~7;
        this.buffer = new ArrayBuffer(PREFIX_SIZE + bufferLen);

        var input_array = new Int32Array(shapes.buffer, shapes.byteOffset, wordLen);
        var output_array = new Int32Array(this.buffer, PREFIX_SIZE, wordLen);
        for (var i = 0; i < output_array.length; ++i) {
            output_array[i] = input_array[i];
        }

        this.asm = module(window, {abort: abort, physTrace: physTrace}, this.buffer);
        this.input = new Int32Array(this.buffer, INPUT_ADDR, IO_SIZE);
        this.output = new Int32Array(this.buffer, OUTPUT_ADDR, IO_SIZE);
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

            this.asm.collide(INPUT_ADDR, OUTPUT_ADDR);

            return ({
                x: this.output[0],
                y: this.output[1],
                z: this.output[2],
                t: this.output[3],
                d: this.output[4],
                type: this.output[5],
            });
        },
    };

    window.ChunkPhysicsAsm = ChunkPhysicsAsm;
})();
