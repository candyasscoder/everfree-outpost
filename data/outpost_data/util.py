import os
import sys

from PIL import Image

from outpost_data.consts import *
import outpost_data.images as I


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


def chop_image_named(img, table, size=TILE_SIZE):
    result = {}
    for i, row in enumerate(table):
        for j, part_name in enumerate(row):
            if part_name is None:
                continue
            x = j * size
            y = i * size
            tile = img.crop((x, y, x + size, y + size))
            result[part_name] = tile
    return result

def chop_terrain(img):
    return chop_image_named(img, TERRAIN_PARTS, TILE_SIZE)

def chop_image(img, size=TILE_SIZE):
    w, h = img.size
    tw = (w + size - 1) // size
    th = (h + size - 1) // size
    return chop_image_named(img, [[(i, j) for i in range(tw)] for j in range(th)], size)

def stack(base, *args):
    img = base.copy()
    for layer in args:
        img.paste(layer, (0, 0), layer)
    return img

def extract(img, pos, size=(1, 1)):
    x, y = pos
    w, h = size

    x *= TILE_SIZE
    y *= TILE_SIZE
    w *= TILE_SIZE
    h *= TILE_SIZE
    return img.crop((x, y, x + w, y + h))


def build_sheet(objs):
    """Build a sprite sheet for fixed-size objects.  Each object should have
    `image` and `id` fields.  The `image` will be copied into the sheet at a
    position based on its `id`.  The width of the sheet in objects (used for
    computing positions from IDs) will be SHEET_PX // obj_width.
    """

    if len(objs) == 0:
        return Image.new('RGBA', (1, 1))

    obj_w, obj_h = objs[0].image.size
    sheet_cols = SHEET_PX // obj_w
    sheet_rows = (len(objs) + sheet_cols - 1) // sheet_cols
    assert sheet_rows * obj_h <= SHEET_PX
    sheet = Image.new('RGBA', (SHEET_PX, sheet_rows * obj_h))

    for o in objs:
        assert o.id < len(objs)
        x = o.id % sheet_cols
        y = o.id // sheet_cols
        sheet.paste(o.image, (x * obj_w, y * obj_h))

    return sheet


def loader(asset_path, name):
    path = os.path.join(asset_path, name)
    return lambda name: I.load(os.path.join(path, name))
