#!/bin/sh

cd "$(dirname "$(dirname "$0")")"

export RUST_BACKTRACE=1
export RUST_LOG=info
mkdir -p logs
python3 bin/wrapper.py 2>&1 | tee logs/server-$(date -Iseconds).log
