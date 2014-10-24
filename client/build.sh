#!/bin/bash
set -e

if [[ -z "$EMSCRIPTEN_HOME" ]]; then
    echo "must set EMSCRIPTEN_HOME with the prefix for emscripten-fastcomp" 1>&2
    exit 1
fi

if [[ -z "$EMSCRIPTEN_EXTRA_PLUGINS" ]]; then
    echo "must set EMSCRIPTEN_EXTRA_PLUGINS with directory containing extra plugins" 1>&2
    exit 1
fi

if [[ -z "$RUST_HOME" ]]; then
    echo "must set RUST_HOME with the prefix for Rust" 1>&2
    exit 1
fi

if [[ -z "$RUST_SRC" ]]; then
    echo "must set RUST_SRC with the path to the Rust source code" 1>&2
    exit 1
fi

mkdir -p build

if ! [[ -f build/libcore.rlib ]] || [[ -n "$rebuild_libcore" ]]; then
    "$RUST_HOME/bin/rustc" \
        --out-dir=build --crate-type=rlib "$RUST_SRC/src/libcore/lib.rs" \
        -O --target=i686-unknown-linux-gnu -Z no-landing-pads -C no-stack-check
fi

# Emit IR for the rust crate
"$RUST_HOME/bin/rustc" \
    --emit=ir -o build/physics.ll \
    --crate-type=staticlib physics.rs \
    --opt-level=3 --target=i686-unknown-linux-gnu -L build \
    -C lto -Z no-landing-pads -C no-stack-check

# Hack up the IR to account for LLVM version mismatch between rust and
# emscripten
sed -e 's/\<\(readonly\|readnone\|cold\)\>//g' \
    -e 's/\<dereferenceable([0-9]*)//g' \
    build/physics.ll >build/physics.clean.ll

# Assemble IR into bitcode
$EMSCRIPTEN_HOME/bin/llvm-as build/physics.clean.ll -o build/physics.bc

# Apply some emscripted-specific transformations
$EMSCRIPTEN_HOME/bin/opt build/physics.bc \
    -load=$EMSCRIPTEN_EXTRA_PLUGINS/BreakStructArguments.so \
    -strip-debug \
    -internalize \
    -internalize-public-api-list=collide,collide_ramp,get_ramp_angle,get_next_ramp_angle,test \
    -break-struct-arguments \
    -globaldce \
    -pnacl-abi-simplify-preopt -pnacl-abi-simplify-postopt \
    -enable-emscripten-cxx-exceptions \
    -o build/physics.opt.bc

# Generate javascript functions from LLVM IR
$EMSCRIPTEN_HOME/bin/llc build/physics.opt.bc \
    -march=js -filetype=asm \
    -emscripten-assertions=1 \
    -emscripten-no-aliasing-function-pointers \
    -emscripten-max-setjmps=20 \
    -O0 \
    -o build/physics.o.js

python handle_function_tables.py <build/physics.o.js >build/physics.o2.js

# Paste the javascript functions into the physics.js template
awk '
    /INSERT_EMSCRIPTEN_FUNCTIONS/ {
        while ((getline < "build/physics.o2.js") > 0) {
            if ($0 == "// EMSCRIPTEN_END_FUNCTIONS")
                break;
            print;
        }
    }
    /INSERT_EMSCRIPTEN_STATIC/ {
        getline < "build/physics.o2.js";
        getline < "build/physics.o2.js";
        start = index($0, "[");
        end = index($0, "]");
        print substr($0, start, end - start + 1);
        getline;
    }
    { print }
    ' physics.tmpl.js >build/physics.js
