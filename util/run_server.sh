#!/bin/sh

cd "$(dirname "$(dirname "$0")")"

rm -f repl control

export RUST_BACKTRACE=1
export RUST_LOG=info
mkdir -p logs
bin/wrapper 2>&1 | tee logs/server-$(date -Iseconds).log
