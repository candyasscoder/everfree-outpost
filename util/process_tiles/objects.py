from itertools import count

from process_tiles import tree
from process_tiles.util import combine_prefix

from process_tiles.blocks import assign_ids

def parse_raw(yaml):
    t = tree.expand_tree(yaml, '/')
    tree.tag_leaf_dicts(t)
    tree.apply_defaults(t, combine={'prefix': combine_prefix})
    objects = tree.flatten_tree(t)

    assign_ids(objects)
    parse_block_refs(objects)
    for k,v in objects.items():
        v['name'] = k

    return objects

def parse_block_refs(objects):
    def handle_part(part, prefix):
        part = part.strip()
        if part == '_':
            return 'empty'
        elif part.startswith('/'):
            return part[1:]
        elif prefix is None:
            return part
        else:
            return prefix + '/' + part

    for name, obj in objects.items():
        prefix = obj.get('prefix')
        grid = []

        # Fill in `grid` with full block paths.
        for z in count(0):
            key = 'z%d' % z
            if key not in obj:
                break
            size_z = z

            layer = []
            for row_str in obj[key]:
                row = list(handle_part(b, prefix) for b in row_str.split(','))
                layer.append(row)
            grid.append(layer)

        # Calculate `size`.
        if len(grid) == 0 or len(grid[0]) == 0:
            size = (0, 0, 0)
        else:
            size = (len(grid[0][0]), len(grid[0]), len(grid))

        # Sanity check: make sure grid is actually rectangular.
        for y, layer in enumerate(grid):
            assert len(layer) == size[1], \
                    'layer y=%d of object %s has wrong number of rows (%d != %d)' % \
                    (y, name, len(layer), size[1])
            for x, row in enumerate(layer):
                assert len(row) == size[0], \
                        'row x=%d, y=%d of object %s has wrong number of items (%d != %d)' % \
                        (x, y, name, len(row), size[0])

        # Flatten `grid` to build `blocks` array.
        blocks = [item for layer in grid for row in layer for item in row]

        obj['size'] = size
        obj['grid'] = grid
        obj['blocks'] = blocks

def build_json(object_arr):
    def go(obj):
        if obj is None:
            return None

        x, y, z = obj['size']
        return {
                'name': obj['name'],
                'size_x': x,
                'size_y': y,
                'size_z': z,
                'blocks': obj['blocks'],
                }

    return [go(o) for o in object_arr]
