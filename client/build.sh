#!/bin/bash
set -e

if [[ -z "$EMSCRIPTEN_HOME" ]]; then
    echo "must set EMSCRIPTEN_HOME with the prefix for emscripten-fastcomp" 1>&2
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
    -O --target=i686-unknown-linux-gnu -L build \
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
    -strip-debug \
    -internalize \
    -internalize-public-api-list=collide \
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

# Paste the javascript functions into the physics.js template
awk '
    /INSERT_EMSCRIPTEN_FUNCTIONS/ {
        while ((getline < "build/physics.o.js") > 0) {
            if ($0 == "// EMSCRIPTEN_END_FUNCTIONS")
                break;
            print;
        }
    }
    { print }
    ' physics.tmpl.js >build/physics.js
