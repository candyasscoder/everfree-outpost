window.asmlibs_code = function(global, env, buffer) {
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
};

window.asmlibs_data = new Uint8Array(
    // INSERT_EMSCRIPTEN_STATIC
);
