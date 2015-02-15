from collections import defaultdict
import sys

from process_tiles import tree

def parse_raw(yaml):
    result = {}
    for filename, dct in yaml.items():
        t = tree.expand_tree(dct)
        tiles = tree.collect_scalars(t)
        expand_regions(tiles)

        for k, v in tiles.items():
            assert k not in result, \
                    'duplicate definition of tile %r' % k

            x, y = v

            result[k] = {
                    'sheet': filename,
                    'x': x,
                    'y': y,
                    'name': k,
                    }

    return result

def expand_regions(tree):
    region_keys = set()

    for k, v in tree.items():
        if len(v) == 2:
            continue
        assert len(v) == 4, \
                'bad length for point/region: %d' % (len(v),)

        region_keys.add(k)

    for k in region_keys:
        base_x, base_y, width, height = tree[k]
        del tree[k]

        for y in range(0, abs(height)):
            for x in range(0, abs(width)):
                real_x = base_x + (x if width > 0 else -width - x - 1)
                real_y = base_y + (y if height > 0 else -height - y - 1)
                tree['%s/%d%d' % (k, x, y)] = [real_x, real_y]
