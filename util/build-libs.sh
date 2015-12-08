#!/bin/bash
# Script to build the required Rust libraries.  Set up a directory with clones
# of Rust and all the libraries listed in README.md, and check out the
# specified revision of each one.  Then set $RUSTC and run this script from
# that directory.  Libraries will be placed in $PWD/lib.
set -e

base=$PWD

build_src() {
    local src=$1
    local crate=$2
    shift 2
    $RUSTC -L $base/lib --out-dir $base/lib --crate-type=lib "$src" \
        -O --crate-name=$crate "$@" \
        --extern libc=$base/lib/liblibc.rlib \
        --extern log=$base/lib/liblog.rlib
}

build() {
    build_src src/lib.rs "$@"
}

in_dir() {
    local dir=$1
    shift 1
    pushd "$dir"
    "$@"
    popd
}

in_dir libc  build_src rust/src/liblibc/lib.rs libc \
    --cfg 'feature="cargo-build"'

in_dir bitflags  build bitflags
in_dir rand  build rand
in_dir rust-memchr  build memchr
in_dir aho-corasick  build aho_corasick
in_dir regex/regex-syntax  build regex_syntax
in_dir regex  build regex
in_dir log  build log
in_dir log/env  build env_logger
in_dir rustc-serialize  build rustc_serialize

in_dir time  build time

in_dir rusqlite/libsqlite3-sys  build libsqlite3_sys -lsqlite3
in_dir rusqlite  build rusqlite

in_dir linked-hash-map  build linked_hash_map
in_dir lru-cache  build lru_cache
