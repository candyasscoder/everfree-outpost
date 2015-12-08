# Everfree Outpost

Dependencies:

 - rust-lang/rust 1.3.0-dev-gdc6e3bb
 - kripken/emscripten-fastcomp 1.34.0-0-gdccd651  (Other Emscripten components
   are not required)
 - epdtry/rust-emscripten-passes eea6274
 - rust-lang/bitflags 274b488
 - rust-lang/rand 164659b
 - rust-lang/regex 0.1.28-82-g4165b3c
 - rust-lang/log fb2d9aa
 - rust-lang/rustc-serialize 376f43a
 - rust-lang/time 79628fa
 - BurntSushi/rust-memchr 0.1.3
 - BurntSushi/aho-corasick 0.3.0
 - jgallagher/rusqlite e896738
 - contain-rs/linked-hash-map 4f944c6
 - contain-rs/lru-cache dc58d49
 - python3
 - python3-pillow
 - python3-yaml
 - liblua5.1
 - ninja
 - closure-compiler
 - yui-compressor

The script `util/build_libs.sh` may be useful for compiling the Rust libraries.

Required environment variables:

 - `EM_FASTCOMP`: path to emscripten-fastcomp build directory (containing `bin/`)
 - `EM_PLUGINS`: path to directory containing rust-emscripten-passes binaries
 - `RUST_SRC`: path to a rust-lang/rust checkout (for building an asm.js
   version of libcore)

Optional environment variables:

 - `RUST_EXTRA_LIBDIR`: extra directory to search for Rust libraries
 - `RUSTC`, `PYTHON3`, `CLOSURE_COMPILER`, `YUI_COMPRESSOR`: override paths for
   various programs used during the build

Additional dependencies for the deployment scripts:

 - ansible
 - s3cmd
