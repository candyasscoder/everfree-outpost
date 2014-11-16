from collections import defaultdict
import sys

from process_tiles import tree
from process_tiles.tree import Leaf

SIDES = ('top', 'bottom', 'front', 'back')

def parse_raw(yaml):
    t = tree.expand_tree(yaml, '/')
    tree.tag_leaf_values(t)
    expand_regions(t)
    tiles = tree.flatten_tree(t)

    tiles = dict(split_filename(k,v) for k,v in tiles.items())

    return tiles

def expand_regions(tree):
    def go(tree):
        for k,v in tree.items():
            if isinstance(v, Leaf):
                tree[k] = expand(v)
            else:
                go(v)

    def expand(leaf):
        if len(leaf.value) == 2:
            return leaf
        assert len(leaf.value) == 4, \
                'bad length for point/region: %d' % (len(leaf.value),)

        base_x, base_y, width, height = leaf.value
        result = {}

        for y in range(0, abs(height)):
            for x in range(0, abs(width)):
                real_x = base_x + (x if width > 0 else -width - x - 1)
                real_y = base_y + (y if height > 0 else -height - y - 1)
                result[str(x) + str(y)] = Leaf([real_x, real_y])

        return result

    go(tree)

def split_filename(k, v):
    sheet_base, _, name = k.partition('.png/')
    x, y = v
    dct = {
            'sheet': sheet_base + '.png',
            'x': x,
            'y': y,
            'name': name,
            }
    return (name, dct)
