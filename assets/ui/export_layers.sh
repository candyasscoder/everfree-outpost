#!/bin/sh
if [ -z "$GIMP_LAYER_EXPORT_DIR" ]; then
    echo 'must set $GIMP_LAYER_EXPORT_DIR' 1>&2
    exit 1
fi
exec gimp --new-instance --no-interface --no-data --no-fonts \
    --batch-interpreter python-fu-eval --batch - "$@" \
    <"$(dirname "$0")/export_layers.py"
