#!/bin/bash
set -e

if [ -z "$version" ]; then
    version=$(date +%Y-%m-%d)
fi

name_src=outpost-$version
name_linux=outpost-linux-$version
name_win32=outpost-win32-$version
name_mod=outpost-mod-win32-$version

mk_src() {
    git archive --format=tar --prefix=${name_src}/ HEAD | xz -c >${name_src}.tar.xz
    tar -xf ${name_src}.tar.xz
}

mk_linux() {
    mkdir build_tmp
    pushd build_tmp
    ../configure \
        --dist-dir=../$name_linux \
        --rustc=$RUSTC \
        --emscripten-fastcomp-prefix=$EM_FASTCOMP \
        --emscripten-passes-prefix=$EM_PLUGINS \
        --rust-extra-libdir=$RUST_EXTRA_LIBDIR \
        --rust-home=$RUST_SRC \
        --bitflags-home=$RUST_EXTRA_LIBDIR/../bitflags \
        --release \
        --mods=$DEFAULT_MODS
    ninja
    popd
    rm -r build_tmp
    tar -cJf ${name_linux}.tar.xz ${name_linux}
}

mk_win32() {
    rsync -av -e ssh "${name_src}/" "$WIN32_HOST:outpost/src/" --delete-after
    rsync -av -e ssh "${name_linux}/" "$WIN32_HOST:outpost/dist-linux/" --delete-after

    scp "$0" $WIN32_HOST:
    ssh $WIN32_HOST bash "$(basename "$0")" build_win32_worker

    rsync -av -e ssh "$WIN32_HOST:outpost/dist-win32/" "${name_win32}/" --delete-after

    cp doc/win32_readme.txt ${name_win32}/README.txt
    zip -r ${name_win32}.zip ${name_win32}
}

build_win32_worker() {
    export PATH=/mingw32/bin:$PATH
    mkdir -p ~/outpost/build
    cd ~/outpost/build

    # TODO: hack
    touch ../dist-linux/util/outpost_savegame.dll

    ../src/configure \
        --dist-dir=../dist-win32 \
        --prebuilt-dir=../dist-linux \
        --use-prebuilt=data,scripts,www,util \
        --rustc=$HOME/rust/i686-pc-windows-gnu/stage2/bin/rustc \
        --rust-extra-libdir=$HOME/outpost-libs/lib \
        --cxxflags=-I$HOME/websocketpp \
        --release \
        --with-server-gui \
        --force
    ninja

    # TODO: more hack
    for lib in libboost_system-mt libgcc_s_dw2-1 libsqlite3-0 \
            libstdc++-6 libwinpthread-1 lua51; do
        cp -v /mingw32/bin/${lib}.dll ../dist-win32/bin
    done
}

mk_mod() {
    mkdir -p ${name_mod}
    for f in `git ls-files assets data gen mk mods scripts util`; do
        mkdir -p ${name_mod}/$(dirname $f)
        cp $f ${name_mod}/$f
    done
    rm -rf ${name_mod}/prebuilt
    cp -r ${name_win32} ${name_mod}/prebuilt
    cp doc/mod_readme.txt ${name_mod}/README.txt
    cp util/build_gui.py ${name_mod}/build_gui.py

    zip -r ${name_mod}.zip ${name_mod}

    rsync -av -e ssh "${name_mod}/" "$WIN32_HOST:outpost/dist-mod-win32/" --delete-after
}

mk_all() {
    mk_src
    mk_linux
    mk_win32
    mk_mod
}

if [ -n "$1" ]; then
    "$1"
else
    mk_all
fi
