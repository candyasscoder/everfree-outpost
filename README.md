# Everfree Outpost

Dependencies:

 - rust-lang/rust 1.0.0-alpha.2-623-gbdf6e4f
 - kripken/emscripten-fastcomp 1.29.0-0-g49ddcf5  (Other Emscripten components
   are not required)
 - epdtry/rust-emscripten-passes eea6274
 - rust-lang/bitflags 63be765
 - Gankro/collect-rs 17365fa
 - rust-lang/log e78b736
 - rust-lang/rand 9561a6a
 - jgallagher/rusqlite 0.0.10-0-g9db251e
 - rust-lang/rustc-serialize 392b17b
 - reem/rust-traverse 69135dd
 - rust-lang/time dc3f310
 - python3
 - python3-pillow
 - python3-tornado
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
