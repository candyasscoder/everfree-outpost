#!/bin/bash
set -e

cd "$(dirname "$0")/.."

dist=outpost-$(date +%Y-%m-%d)

make RELEASE=1 DIST=$dist

tar -cJf ${dist}.tar.xz ${dist}
