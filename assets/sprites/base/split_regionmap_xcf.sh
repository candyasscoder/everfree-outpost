#!/bin/sh
set -e

# Expected layer arrangement:
#  + Layer Group (region maps)
#    - hat box
#    - frontwing
#    - horn
#    - base
#    - backwing
#  - equip_f_hat
#  - frontwing
#  - horn
#  - base
#  - backwing
# Layers are numbered from 0 starting at the bottom.

img=$(basename "$1" .xcf)
convert "$1" "${img}_split.png"
mv "${img}_split-0.png" "${img}-backwing.png"
mv "${img}_split-1.png" "${img}-base.png"
mv "${img}_split-2.png" "${img}-horn.png"
mv "${img}_split-3.png" "${img}-frontwing.png"
mv "${img}_split-5.png" "${img}-backwing-regions.png"
mv "${img}_split-6.png" "${img}-base-regions.png"
mv "${img}_split-7.png" "${img}-horn-regions.png"
mv "${img}_split-8.png" "${img}-frontwing-regions.png"
mv "${img}_split-9.png" "${img}-hat-box.png"
rm "${img}"_split*.png
