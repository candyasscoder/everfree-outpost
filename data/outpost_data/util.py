import sys

from outpost_data.consts import *


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


def chop_image_named(img, table):
    result = {}
    for i, row in enumerate(table):
        for j, part_name in enumerate(row):
            x = j * TILE_SIZE
            y = i * TILE_SIZE
            tile = img.crop((x, y, x + TILE_SIZE, y + TILE_SIZE))
            result[part_name] = tile
    return result

def chop_terrain(img):
    return chop_image_named(img, TERRAIN_PARTS)

def chop_image(img):
    w, h = img.size
    tw = (w + TILE_SIZE - 1) // TILE_SIZE
    th = (h + TILE_SIZE - 1) // TILE_SIZE
    return chop_image_named(img, [[(i, j) for i in range(tw)] for j in range(th)])

def stack(base, *args):
    img = base.copy()
    for layer in args:
        img.paste(layer, (0, 0), layer)
    return img
