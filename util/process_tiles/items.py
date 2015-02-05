from itertools import count

from process_tiles import tree
from process_tiles.util import combine_prefix

from process_tiles.blocks import assign_ids

def parse_raw(yaml):
    t = tree.expand_tree(yaml, '/.')
    tree.tag_leaf_dicts(t)
    tree.apply_defaults(t, combine={'prefix': combine_prefix})
    items = tree.flatten_tree(t)

    assign_ids(items)
    parse_tile_refs(items)
    for k,v in items.items():
        v['name'] = k

    return items

def parse_tile_refs(items):
    def handle_part(part, prefix):
        part = part.strip()
        if part.startswith('/'):
            return part[1:]
        elif prefix is None:
            return part
        else:
            return prefix + '/' + part

    for name, item in items.items():
        prefix = item.get('prefix')
        parts = item['tile'].split('+')
        item['tile'] = tuple(handle_part(p, prefix) for p in parts)

def build_client_json(item_arr, atlas):
    def go(item):
        if item is None:
            return None

        name = item['name']
        ui_name = item['ui_name']
        tile = atlas[item['tile']]

        return {
                'name': name,
                'ui_name': ui_name,
                'tile': tile,
                }

    return [go(i) for i in item_arr]

def build_server_json(item_arr):
    def go(item):
        if item is None:
            return None

        name = item['name']

        return {
                'name': name,
                }

    return [go(i) for i in item_arr]
