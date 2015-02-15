from itertools import count

from process_tiles import tree
from process_tiles.util import combine_prefix, apply_prefix, assign_ids

def parse_raw(yaml):
    t = tree.expand_tree(yaml)
    tree.apply_defaults(t)
    tree.propagate(t, combine={'prefix': combine_prefix})
    items = tree.collect_dicts(t,
            fields=('tile', 'ui_name'))

    assign_ids(items, 'items')
    parse_tile_refs(items)
    for k,v in items.items():
        v['name'] = k

    return items

def parse_tile_refs(items):
    for name, item in items.items():
        if 'tile' not in item:
            item['tile'] = ()
            continue

        prefix = item.get('prefix')
        parts = item['tile'].split('+')
        item['tile'] = tuple(apply_prefix(prefix, p.strip()) for p in parts)

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
