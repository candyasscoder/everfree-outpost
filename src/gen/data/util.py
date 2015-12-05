import functools
import hashlib
import importlib
import inspect
import os
import sys

from PIL import Image

from .consts import *


def cached(f):
    inited = False
    value = None

    @functools.wraps(f)
    def g():
        nonlocal inited, value
        if not inited:
            value = f()
            inited = True
        return value
    return g


def assign_ids(objs, reserved=None):
    '''Assign a unique ID to every object in `objs`.  This function sets the
    `o.id` field of each object, sorts `objs` by ID, and returns a dict mapping
    names to IDs.
    '''
    if reserved is None:
        special = []
        normal = objs
    else:
        normal = []
        reserved_map = {k: None for k in reserved}
        for o in objs:
            if o.name in reserved_map:
                assert reserved_map[o.name] is None, \
                        'duplicate entries for reserved name %r' % o.name
                reserved_map[o.name] = o
            else:
                normal.append(o)
        special = [reserved_map[k] for k in reserved if reserved_map[k] is not None]

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

def warn(s):
    sys.stderr.write('warning: ' + s + '\n')


def chop_image_named(img, table, size=(TILE_SIZE, TILE_SIZE)):
    sx, sy = size
    result = {}
    for i, row in enumerate(table):
        for j, part_name in enumerate(row):
            if part_name is None:
                continue
            x = j * sx
            y = i * sy
            tile = img.crop((x, y, x + sx, y + sy))
            result[part_name] = tile
    return result

def chop_terrain(img):
    return chop_image_named(img, TERRAIN_PARTS, (TILE_SIZE, TILE_SIZE))

def chop_image(img, size=(TILE_SIZE, TILE_SIZE)):
    w, h = img.size
    sx, sy = size
    tw = (w + sx - 1) // sx
    th = (h + sy - 1) // sy
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


class Page(object):
    def __init__(self, size):
        self.w, self.h = size
        self.in_use = 0
        self.avail_area = self.w * self.h

    def place(self, box):
        w, h = box

        base_mask = (1 << w) - 1
        mask = 0
        for i in range(h):
            mask |= base_mask << (i * self.w)

        for y in range(0, self.h - h + 1):
            base_i = y * self.w
            for x in range(0, self.w - w + 1):
                i = base_i + x
                if (mask << i) & self.in_use == 0:
                    self.in_use |= mask << i
                    self.avail_area -= w * h
                    return (x, y)

        return None

def pack_boxes(page_size, boxes):
    """Pack a list of boxes (`(w, h)` pairs) into pages of size `page_size`
    (`(page_w, page_h)`).  Returns the number of generated pages and a list
    containing `(page_idx, (x, y))` for each input box."""
    def key(b):
        i, (w, h) = b
        return (w * h, h, w)
    # Sort by decreasing size.
    boxes = sorted(enumerate(boxes), key=key, reverse=True)
    result = [None] * len(boxes)

    pages = [Page(page_size)]
    for i, box in boxes:
        w,h = box
        for j, p in reversed(list(enumerate(pages))):
            if p.avail_area < w * h:
                continue
            pos = p.place(box)
            if pos is not None:
                result[i] = (j, pos)
                break
        else:
            # The loop didn't `break`, so the box didn't fit on any existing page.
            pages.append(Page(page_size))
            pos = pages[-1].place(box)
            assert pos is not None, \
                    'box is too large to fit on a page (%s > %s)' % (box, page_size)
            result[i] = (len(pages) - 1, pos)

    assert not any(r is None for r in result)
    return len(pages), result

def pack_boxes_uniform(page_size, n):
    """Like `pack_boxes`, but for `n` boxes each of size `(1, 1)`."""
    w, h = page_size
    num_per_page = w * h

    def go():
        for i in range(n):
            page = i // num_per_page

            idx = i % num_per_page
            x = idx % w
            y = idx // w

            yield page, (x, y)

    return (n + num_per_page - 1) // num_per_page, go()

def build_sheets(imgs, offsets, num_pages, page_size, scale):
    """Given a list of images and the output of `pack_boxes`, paste the images
    together into `num_pages` sheets.  The `scale` argument (`(sx, sy)`) gives
    the pixel size of a (1, 1) `pack_boxes` box."""
    sx, sy = scale if isinstance(scale, tuple) else (scale, scale)
    pw, ph = page_size

    sheets = [Image.new('RGBA', (pw * sx, ph * sy)) for _ in range(num_pages)] 

    for img, (page, (x, y)) in zip(imgs, offsets):
        sheets[page].paste(img, (x * sx, y * sy))

    return sheets

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
    sheet = Image.new('RGBA', (SHEET_PX, SHEET_PX))

    for o in objs:
        assert o.id < len(objs)
        x = o.id % sheet_cols
        y = o.id // sheet_cols
        sheet.paste(o.image, (x * obj_w, y * obj_h))

    return sheet


def dedupe_images(imgs):
    """Deduplicate a set of images.  Returns a list of images and a dict
    `mapping id(i)` to the index of (an image identical to) `i` in the list."""
    idx_map = {}
    result = []

    # Maps hash(i) to an association list of (i, val) pairs.
    hash_map = {}
    def find_or_insert(i, val):
        """Return the value in `hash_map` for image `i`, or insert `val` as the
        value and return `None`."""
        h = 0
        for x in i.getdata():
            h = (h * 37 + hash(x)) & 0xffffffff

        if h in hash_map:
            # Check every image in the selected bucket.
            for (i2, val2) in hash_map[h]:
                if i.size == i2.size and i.mode == i2.mode and \
                        all(a == b for a,b in zip(i.getdata(), i2.getdata())):
                    return val2

            # Not in the bucket, so add it.
            hash_map[h].append((i, val))
            return None
        else:
            # Create a new bucket for this image.
            hash_map[h] = [(i, val)]
            return None

    for i in imgs:
        if id(i) not in idx_map:
            next_idx = len(result)
            old_idx = find_or_insert(i, next_idx)
            if old_idx is None:
                # Wasn't found in the hash map.  Add to `result`.
                idx_map[id(i)] = next_idx
                result.append(i)
            else:
                idx_map[id(i)] = old_idx

    return result, idx_map

def dedupe(vals, keys):
    """Deduplicate a list of objects using their pre-computed unique keys."""
    key_idx = {}
    val_idx = {}
    result = []

    for k,v in zip(keys, vals):
        if k in key_idx:
            val_idx[id(v)] = key_idx[k]
        else:
            next_idx = len(result)
            result.append(v)
            key_idx[k] = next_idx
            val_idx[id(v)] = next_idx

    return result, val_idx

def hash_image(i):
    b = bytes(x for p in i.getdata() for x in p)
    return hashlib.sha1(b).hexdigest()

def extract_mod_name(module_name):
    if module_name.startswith('outpost_data.'):
        parts = module_name.split('.')
        if parts[1] != 'core':
            return parts[1]
    return None

def get_caller_mod_name():
    stack = inspect.stack()
    try:
        for frame in stack[1:]:
            module = inspect.getmodule(frame[0])
            if module is None:
                continue
            mod_name = extract_mod_name(module.__name__)
            if mod_name is not None:
                return mod_name
        raise ValueError("couldn't detect calling module name")
    finally:
        del stack

def project(p):
    x, y, z = p
    return (x, y - z)
