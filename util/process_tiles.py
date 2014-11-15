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

SIDES = ('top', 'bottom', 'front', 'back')


def assert_no_overlap(d1, d2, msg):
    overlap = set(d1.keys()) & set(d2.keys())
    assert not overlap, '%s: %s' % (msg, sorted(overlap))

def process_block_group(g, prefix=''):
    blocks = defaultdict(dict)
    subgroup_blocks = {}

    if '_.prefix' in g:
        prefix = prefix + '/' + g['_.prefix']

    for k, v in g.items():
        if isinstance(v, dict):
            group_name = k
            new_blocks = process_block_group(v, prefix)
            new_blocks = dict(('%s/%s' % (group_name, k), v) for k,v in new_blocks.items())
            subgroup_blocks.update(new_blocks)
            continue

        name, _, field = k.partition('.')

        # Make sure blocks[name] gets created, even if it's left empty.
        block = blocks[name]

        if field == '':
            if v is not None:
                assert_no_overlap(v, block,
                        'multiple definitions of fields for %s' % name)
                block.update(v)
        else:
            assert field not in block, 'multiple definitions of %s.%s' % (name, field)
            block[field] = v

    defaults = blocks.pop('_', {})

    blocks = dict(blocks.items())
    assert_no_overlap(blocks, subgroup_blocks,
            'blocks are defined in multiple places')
    blocks.update(subgroup_blocks)

    for name, block in blocks.items():
        for field, value in defaults.items():
            if field not in block:
                block[field] = value

        for side in SIDES:
            if side not in block:
                continue
            if isinstance(block[side], tuple):
                continue

            def handle(layer):
                layer = layer.strip()
                if layer.startswith('/'):
                    return layer
                else:
                    return prefix + '/' + layer

            block[side] = tuple(handle(layer) for layer in block[side].split('+'))

    return blocks

def process_tiles(g):
    def go(g):
        tiles = {}
        subgroup_tiles = {}

        for k, v in g.items():
            if isinstance(v, dict):
                group_name = k
                new_tiles = go(v)
                new_tiles = dict((group_name + '/' + k, v) for k,v in new_tiles.items())
                subgroup_tiles.update(new_tiles)
                continue

            if len(v) == 2:
                tiles[k] = v
            elif len(v) == 4:
                base_x, base_y, width, height = v

                for y in range(0, abs(height)):
                    for x in range(0, abs(width)):
                        real_x = base_x + (x if width > 0 else -width - x - 1)
                        real_y = base_y + (y if height > 0 else -height - y - 1)
                        name = k + '/' + str(x) + str(y)
                        assert name not in tiles, \
                                'tile %r is defined in region and also alone' % name
                        tiles[name] = [real_x, real_y]
            else:
                assert False, \
                        'expected 2 or 4 items in tile definition, but found %d' % len(v)

        assert_no_overlap(tiles, subgroup_tiles,
                'tiles are defined in multiple places')
        tiles.update(subgroup_tiles)
        return tiles

    raw_tiles = go(g)
    tiles = {}
    for k, v in raw_tiles.items():
        sheet_base, _, name = k.partition('.png/')
        sheet = sheet_base + '.png'
        x, y = v
        tiles['/' + name] = {'sheet': sheet, 'x': x, 'y': y}
    return tiles

def build_array(blocks):
    blocks = dict((k, dict(v)) for k,v in blocks.items())
    names = sorted(blocks.keys())

    # Assign ids for blocks with `id_base` set, and check for id collisions.
    used_ids = {}
    base_counters = {}
    for name, block in ((n, blocks[n]) for n in names):
        if 'id_base' in block:
            base = block['id_base']
            if base not in base_counters:
                base_counters[base] = base
            block['id'] = base_counters[base]
            del block['id_base']
            base_counters[base] += 1

        if 'id' not in block:
            continue

        id = block['id']
        assert id not in used_ids, \
                'id collision: %d = %s, %s' % (id, used_ids[id], name)
        used_ids[id] = name

    # Now assign ids to blocks without `id` or `id_base`.
    next_id = 0
    for name, block in ((n, blocks[n]) for n in names):
        if 'id' in block:
            continue

        while next_id in used_ids:
            next_id += 1
        block['id'] = next_id
        used_ids[block['id']] = name
        next_id += 1

    # Generate the final list.
    num_ids = 1 + max(id for id in used_ids.keys())
    block_array = [None] * num_ids
    for id, name in used_ids.items():
        block = blocks[name]

        block['name'] = name
        if 'shape' in block:
            block['shape'] = SHAPE_ID[block['shape']]

        block_array[id] = block

    occupancy = len(used_ids) / num_ids
    if occupancy < 0.8:
        sys.stderr.write('warning: low block array occupancy (%.1f%%)\n' %
                (occupancy * 100))

    return block_array

def compute_atlas(blocks, tiles):
    order = [()]
    rev = {}

    for block in blocks:
        for k in SIDES:
            if k not in block:
                continue

            tile_names = block[k];
            layers = tuple(tiles[tile_name] for tile_name in tile_names)
            display = tuple((t['sheet'], t['x'], t['y']) for t in layers)

            if tile_names in rev:
                continue
            order.append(display)
            rev[tile_names] = len(order) - 1

    return (order, rev)

TILE_SIZE = 32
ATLAS_WIDTH = 32

def build_atlas(order, in_dir, out_file):
    sheets = dict((s, None) for layers in order for s,_,_ in layers)
    for s in sheets:
        if s is None:
            continue
        sheets[s] = Image.open(os.path.join(in_dir, s))

    atlas_height = (len(order) + ATLAS_WIDTH - 1) // ATLAS_WIDTH
    px_width = ATLAS_WIDTH * TILE_SIZE
    px_height = atlas_height * TILE_SIZE
    output = Image.new('RGBA', (px_width, px_height))

    for idx, layers in enumerate(order):
        out_tx = idx % ATLAS_WIDTH
        out_ty = idx // ATLAS_WIDTH

        for (s, tx, ty) in layers:
            if sheets[s] is None:
                continue

            in_x = tx * TILE_SIZE
            in_y = ty * TILE_SIZE
            display = sheets[s].crop((in_x, in_y, in_x + TILE_SIZE, in_y + TILE_SIZE))
            output.paste(display, (out_tx * TILE_SIZE, out_ty * TILE_SIZE), mask=display)

    output.save(out_file)
    
def build_json(blocks, atlas_rev):
    result = []
    for block in blocks:
        item = {'shape': block['shape']}
        for side in SIDES:
            if side in block:
                item[side] = atlas_rev[block[side]]
        result.append(item)
    return result

if __name__ == '__main__':
    atlas_file = None
    atlas_input_dir = None
    positional = []
    for opt in sys.argv[1:]:
        name, _, value = opt.partition('=')
        if name == '--gen-atlas-file':
            atlas_file = value
        elif name == '--atlas-input-dir':
            atlas_input_dir = value
        else:
            positional.append(opt)

    tiles_file, blocks_file = positional

    if atlas_file is not None and atlas_input_dir is None:
        sys.stderr.write('--gen-atlas-file requires --atlas-input-dir')
        sys.exit(1)

    with open(tiles_file, 'r') as f:
        tiles = process_tiles(yaml.load(f))
    with open(blocks_file, 'r') as f:
        blocks = process_block_group(yaml.load(f))
    block_array = build_array(blocks)
    atlas_order, atlas_rev = compute_atlas(block_array, tiles)

    if atlas_file is not None:
        build_atlas(atlas_order, atlas_input_dir, atlas_file)

    raw_json = {
            'blocks': build_json(block_array, atlas_rev),
            'atlas_width': ATLAS_WIDTH,
            }
    json.dump(raw_json, sys.stdout)
