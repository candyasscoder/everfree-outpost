from collections import defaultdict
import sys

from process_tiles import tree
from process_tiles.util import combine_prefix

SIDES = ('top', 'bottom', 'front', 'back')

def parse_raw(yaml):
    t = tree.expand_tree(yaml, '/.')
    tree.tag_leaf_dicts(t)
    tree.apply_defaults(t, combine={'prefix': combine_prefix})
    blocks = tree.flatten_tree(t)

    assign_ids(blocks)
    parse_tile_refs(blocks)
    for k,v in blocks.items():
        v['name'] = k

    return blocks

def assign_ids(blocks):
    sorted_names = sorted(blocks.keys())
    sorted_blocks = [blocks[n] for n in sorted_names]

    # Assign IDs to blocks with 'id_base' but no 'id'.
    base_counters = defaultdict(lambda: 0)
    for block in sorted_blocks:
        if 'id' in block or 'id_base' not in block:
            continue
        id_base = block['id_base']
        block['id'] = id_base + base_counters[id_base]
        base_counters[id_base] += 1

    # Assign ids to remaining blocks.
    used_ids = set(block.get('id') for block in sorted_blocks)
    used_ids.remove(None)

    next_id = 0
    for block in sorted_blocks:
        if 'id' in block:
           continue

        while next_id in used_ids:
           next_id += 1
        block['id'] = next_id
        used_ids.add(next_id)

    # Check for duplicate ids.
    seen_ids = defaultdict(set)
    for name in sorted_names:
        seen_ids[blocks[name]['id']].add(name)
    for k in sorted(seen_ids.keys()):
        vs = seen_ids[k]
        if len(vs) > 1:
            raise ValueError('blocks %s share id %r' % (sorted(vs), k))

def parse_tile_refs(blocks):
    def handle_part(part, prefix):
        part = part.strip()
        if part.startswith('/'):
            return part[1:]
        elif prefix is None:
            return part
        else:
            return prefix + '/' + part

    for block in blocks.values():
        prefix = block.get('prefix')
        for side in SIDES:
            if side not in block:
                continue

            parts = block[side].split('+')
            block[side] = tuple(handle_part(p, prefix) for p in parts)


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

def build_client_json(block_arr, atlas):
    def go(block):
        if block is None:
            return None

        result = {}
        for side in SIDES:
            if side in block:
                result[side] = atlas[block[side]]

        assert 'shape' in block, \
                'no shape is defined for block %r' % (block['name'],)
        result['shape'] = SHAPE_ID[block['shape']]

        return result

    return [go(b) for b in block_arr]

def build_server_json(block_arr):
    def go(block):
        if block is None:
            return None

        return {
                'name': block['name'],
                'shape': block['shape'],
                }

    return [go(b) for b in block_arr]
