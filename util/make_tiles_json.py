from collections import defaultdict
import sys

import json
import yaml

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

def generate(tiles):
    sheets = sorted(set(t['sheet'] for t in tiles.values() if 'sheet' in t))
    sheet_id = dict((sheets[i], i) for i in range(len(sheets)))

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
        del tile['id']
        if 'sheet' in tile:
            tile['sheet'] = sheet_id[tile['sheet']]
        if 'shape' in tile:
            tile['shape'] = SHAPE_ID[tile['shape']]

        tile_array[id] = tile

    occupancy = len(used_ids) / num_ids
    if occupancy < 0.8:
        sys.stderr.write('warning: low tile array occupancy (%.1f%%)\n' %
                (occupancy * 100))

    return (sheets, tile_array)

if __name__ == '__main__':
    raw_yaml = yaml.load(sys.stdin)
    tiles = process_group(raw_yaml)
    sheets, tile_array = generate(tiles)
    raw_json = { 'sheets': sheets, 'tiles': tile_array }
    json.dump(raw_json, sys.stdout)
