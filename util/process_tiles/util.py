def combine_prefix(new, old):
    if old is None:
        return new
    elif new is None:
        return old
    else:
        return old + '/' + new

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
