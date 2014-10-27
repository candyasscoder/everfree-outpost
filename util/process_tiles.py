from collections import defaultdict
import os
import sys

import json
import yaml
from PIL import Image

from pprint import pprint


SHAPE_ID = {
        'empty': 0,
        'floor': 1,
        'solid': 2,
        'ramp_e': 3,
        'ramp_w': 4,
        'ramp_s': 5,
        'ramp_n': 6,
        'ramp_top': 7,
        }


def assert_no_overlap(d1, d2, msg):
    overlap = set(d1.keys()) & set(d2.keys())
    assert not overlap, '%s: %s' % (msg, sorted(overlap))

def process_group(g):
    tiles = defaultdict(dict)
    subgroup_tiles = {}

    for k, v in g.items():
        if k.startswith('/'):
            group_name = k[1:]
            new_tiles = process_group(v)
            new_tiles = dict(('%s/%s' % (group_name, k), v) for k,v in new_tiles.items())
            subgroup_tiles.update(new_tiles)
            continue

        name, _, field = k.partition('.')

        # Make sure tiles[name] gets created, even if it's left empty.
        tile = tiles[name]

        if field == '':
            if v is not None:
                assert_no_overlap(v, tile,
                        'multiple definitions of fields for %s' % name)
                tile.update(v)
        else:
            assert field not in tile, 'multiple definitions of %s.%s' % (name, field)
            tile[field] = v

    defaults = tiles.pop('_', {})

    tiles = dict(tiles.items())
    tiles.update(subgroup_tiles)

    for name, tile in tiles.items():
        for field, value in defaults.items():
            if field not in tile:
                tile[field] = value

    return tiles

def build_array(tiles):
    tiles = dict((k, dict(v)) for k,v in tiles.items())
    names = sorted(tiles.keys())

    # Assign ids for tiles with `id_base` set, and check for id collisions.
    used_ids = {}
    base_counters = {}
    for name, tile in ((n, tiles[n]) for n in names):
        if 'id_base' in tile:
            base = tile['id_base']
            if base not in base_counters:
                base_counters[base] = base
            tile['id'] = base_counters[base]
            del tile['id_base']
            base_counters[base] += 1

        if 'id' not in tile:
            continue

        id = tile['id']
        assert id not in used_ids, \
                'id collision: %d = %s, %s' % (id, used_ids[id], name)
        used_ids[id] = name

    # Now assign ids to tiles without `id` or `id_base`.
    next_id = 0
    for name, tile in ((n, tiles[n]) for n in names):
        if 'id' in tile:
            continue

        while next_id in used_ids:
            next_id += 1
        tile['id'] = next_id
        used_ids[tile['id']] = name
        next_id += 1

    # Generate the final list.
    num_ids = 1 + max(id for id in used_ids.keys())
    tile_array = [None] * num_ids
    for id, name in used_ids.items():
        tile = tiles[name]

        tile['name'] = name
        if 'shape' in tile:
            tile['shape'] = SHAPE_ID[tile['shape']]

        tile_array[id] = tile

    occupancy = len(used_ids) / num_ids
    if occupancy < 0.8:
        sys.stderr.write('warning: low tile array occupancy (%.1f%%)\n' %
                (occupancy * 100))

    return tile_array

def compute_atlas(tiles):
    order = [(None, 0, 0)]
    rev = {}

    for tile in tiles:
        for k in ('top', 'bottom', 'front', 'back'):
            if k not in tile:
                continue
            j, i = tile[k]
            sheet = tile['sheet']
            display = (sheet, j, i)
            if display in rev:
                continue
            order.append(display)
            rev[display] = len(order) - 1

    return (order, rev)

TILE_SIZE = 32
ATLAS_WIDTH = 32

def build_atlas(order, in_dir, out_file):
    sheets = dict((s, None) for s,_,_ in order)
    for s in sheets:
        if s is None:
            continue
        sheets[s] = Image.open(os.path.join(in_dir, s))

    atlas_height = (len(order) + ATLAS_WIDTH - 1) // ATLAS_WIDTH
    px_width = ATLAS_WIDTH * TILE_SIZE
    px_height = atlas_height * TILE_SIZE
    output = Image.new('RGBA', (px_width, px_height))

    for idx, (s, j, i) in enumerate(order):
        if sheets[s] is None:
            continue
        out_j = idx % ATLAS_WIDTH
        out_i = idx // ATLAS_WIDTH

        in_x = j * TILE_SIZE
        in_y = i * TILE_SIZE
        display = sheets[s].crop((in_x, in_y, in_x + TILE_SIZE, in_y + TILE_SIZE))
        output.paste(display, (out_j * TILE_SIZE, out_i * TILE_SIZE))

    output.save(out_file)
    
def build_json(tiles, atlas_rev):
    result = []
    for tile in tiles:
        def get(side):
            if side not in tile:
                return 0
            j,i = tile[side]
            return atlas_rev[(tile['sheet'], j, i)]

        item = {'shape': tile['shape']}
        for side in ('top', 'bottom', 'front', 'back'):
            item[side] = get(side)
        result.append(item)
    return result

if __name__ == '__main__':
    atlas_file = None
    atlas_input_dir = None
    for opt in sys.argv[1:]:
        name, _, value = opt.partition('=')
        if name == '--gen-atlas-file':
            atlas_file = value
        elif name == '--atlas-input-dir':
            atlas_input_dir = value
        else:
            sys.stderr.write('unrecognized option: %r' % name)
            sys.exit(1)

    if atlas_file is not None and atlas_input_dir is None:
        sys.stderr.write('--gen-atlas-file requires --atlas-input-dir')
        sys.exit(1)

    raw_yaml = yaml.load(sys.stdin)
    tiles = process_group(raw_yaml)
    tile_array = build_array(tiles)
    atlas_order, atlas_rev = compute_atlas(tile_array)

    if atlas_file is not None:
        build_atlas(atlas_order, atlas_input_dir, atlas_file)

    raw_json = {
            'tiles': build_json(tile_array, atlas_rev),
            'atlas_width': ATLAS_WIDTH,
            }
    json.dump(raw_json, sys.stdout)
