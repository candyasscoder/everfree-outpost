from collections import defaultdict
import sys

from process_tiles import tree
from process_tiles.util import combine_prefix, apply_prefix, assign_ids

SIDES = ('top', 'bottom', 'front', 'back')

def parse_raw(yaml):
    t = tree.expand_tree(yaml, '/.')
    tree.apply_defaults(t)
    tree.propagate(t, combine={'prefix': combine_prefix, 'shape': tree.default_merge})
    blocks = tree.collect_dicts(t,
            fields=('shape',))

    assign_ids(blocks, 'blocks')
    parse_tile_refs(blocks)
    for k,v in blocks.items():
        v['name'] = k

    return blocks

def parse_tile_refs(blocks):
    for block in blocks.values():
        prefix = block.get('prefix')
        for side in SIDES:
            if side not in block:
                continue

            parts = block[side].split('+')
            block[side] = tuple(apply_prefix(prefix, p.strip()) for p in parts)


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
