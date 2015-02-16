from collections import defaultdict

import process_tiles.tree as tree

def combine_prefix(parent, child):
    if parent is tree.MISSING:
        parent = None
    if child is tree.MISSING:
        child = None

    if child is None:
        return parent
    elif child.startswith('/'):
        return child[1:]
    elif child.startswith('./'):
        child = child[2:]

    if parent is None:
        return child
    else:
        return '%s/%s' % (parent, child)

def apply_prefix(prefix, path):
    if path.startswith('/'):
        return path[1:]
    elif path.startswith('./'):
        path = path[2:]

    if prefix is None:
        return path
    else:
        return '%s/%s' % (prefix, path)


def build_array(items):
    # Generate the final list.
    num_ids = 1 + max(x['id'] for x in items.values())
    array = [None] * num_ids
    for name, item in items.items():
        assert array[item['id']] is None, \
                'overlapping ids: %r, %r' % (array[item['id']]['name'], name)
        array[item['id']] = item

    occupancy = len(items) / num_ids
    if occupancy < 0.8:
        sys.stderr.write('warning: low array occupancy (%.1f%%)\n' %
                (occupancy * 100))

    return array

def build_name_map(items):
    return dict((x['name'], x) for x in items.values())


def assign_ids(items, what):
    sorted_names = sorted(items.keys())
    sorted_items = [items[n] for n in sorted_names]

    # Assign IDs to items with 'id_base' but no 'id'.
    base_counters = defaultdict(lambda: 0)
    for item in sorted_items:
        if 'id' in item or 'id_base' not in item:
            continue
        id_base = item['id_base']
        item['id'] = id_base + base_counters[id_base]
        base_counters[id_base] += 1

    # Assign ids to remaining items.
    used_ids = set(item.get('id') for item in sorted_items)
    used_ids.remove(None)

    next_id = 0
    for item in sorted_items:
        if 'id' in item:
           continue

        while next_id in used_ids:
           next_id += 1
        item['id'] = next_id
        used_ids.add(next_id)

    # Check for duplicate ids.
    seen_ids = defaultdict(set)
    for name in sorted_names:
        seen_ids[items[name]['id']].add(name)
    for k in sorted(seen_ids.keys()):
        vs = seen_ids[k]
        if len(vs) > 1:
            raise ValueError('%s %s share id %r' % (what, sorted(vs), k))
