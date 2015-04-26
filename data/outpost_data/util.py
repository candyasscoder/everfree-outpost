import sys


def assign_ids(objs, reserved=None):
    '''Assign a unique ID to every object in `objs`.  This function sets the
    `o.id` field of each object, sorts `objs` by ID, and returns a dict mapping
    names to IDs.
    '''
    if reserved is None:
        special = []
        normal = objs
    else:
        special = []
        normal = []
        for o in objs:
            if o.name in reserved:
                special.append(o)
            else:
                normal.append(o)

    # Leave `special` in its original order.
    normal.sort(key=lambda o: o.name)

    i = 0

    for o in special:
        o.id = i
        i += 1

    for o in normal:
        o.id = i
        i += 1

    objs.sort(key=lambda o: o.id)
    return dict((o.name, o.id) for o in objs)


SAW_ERROR = False

def err(s):
    global SAW_ERROR
    SAW_ERROR = True
    sys.stderr.write('error: ' + s + '\n')
