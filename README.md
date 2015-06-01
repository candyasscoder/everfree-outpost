# Everfree Outpost

Dependencies:

 - rust-lang/rust 1.2.0-dev-g60926b8
 - kripken/emscripten-fastcomp 1.29.0-0-g49ddcf5  (Other Emscripten components
   are not required)
 - epdtry/rust-emscripten-passes eea6274
 - rust-lang/bitflags ec6b3b5
 - rust-lang/rand 916642f
 - rust-lang/regex 0.1.28-36-g7a72b1f
 - rust-lang/log 07fbe6b
 - rust-lang/rustc-serialize 7900641
 - rust-lang/time c3b0bb3
 - jgallagher/rusqlite 0.0.17-29-g255e5f0
 - contain-rs/linked-hash-map eb4a8cc
 - contain-rs/lru-cache a0dcc31
 - python3
 - python3-pillow
 - python3-yaml
 - liblua5.1

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
