from PIL import Image

from outpost_data.consts import *


class Shape(object):
    def __init__(self, x, y, z, arr):
        self.size = (x, y, z)
        self.shape_array = arr

def floor(x, y, z):
    arr = ['floor'] * (x * y) + ['empty'] * (x * y * (z - 1))
    return Shape(x, y, z, arr)

def solid(x, y, z):
    arr = ['solid'] * (x * y * z)
    return Shape(x, y, z, arr)


class StructureDef(object):
    def __init__(self, name, image, depthmap, shape, layer):
        self.name = name
        self.image = image
        self.depthmap = depthmap
        assert image.size == depthmap.size
        self.size = shape.size
        self.shape = shape.shape_array
        self.layer = layer

        self.light_pos = None
        self.light_color = None
        self.light_radius = None

        self.id = None
        self.sheet_idx = None
        self.offset = None

    def get_display_size(self):
        w, h = self.image.size
        return ((w + TILE_SIZE - 1) // TILE_SIZE,
                (h + TILE_SIZE - 1) // TILE_SIZE)

    def set_light(self, pos, color, radius):
        self.light_pos = pos
        self.light_color = color
        self.light_radius = radius


# Sprite sheets

def build_sheets(structures):
    '''Build sprite sheet(s) containing all the images and depthmaps for the
    provided structures.  This also updates each structure's `sheet_idx` and
    `offset` field with its position in the generated sheet(s).
    '''
    def key(s):
        w, h = s.image.size
        return (w * h, h, w)
    structures = sorted(structures, key=key, reverse=True)

    sheets = [SheetBuilder(0, *SHEET_SIZE)]

    for s in structures:
        size = s.get_display_size()

        sheet, offset = (None, None)
        for sh in sheets[-3:]:
            offset = sh.place(*size)
            if offset is not None:
                sheet = sh
                break

        if offset is None:
            sheet = SheetBuilder(len(sheets), *SHEET_SIZE)
            sheets.append(sheet)
            offset = sheet.place(*size)

        assert offset is not None, \
                'structure %s is too big for a sheet %s' % (s.name, size)

        x, y = offset
        x *= TILE_SIZE
        y *= TILE_SIZE

        px_w, px_h = s.image.size
        if px_h % TILE_SIZE != 0:
            y += 32 - px_h % TILE_SIZE

        sheet.image.paste(s.image, (x, y))
        sheet.depthmap.paste(s.depthmap, (x, y))

        s.sheet_idx = sheet.idx
        s.offset = (x, y)

    return [(s.image, s.depthmap) for s in sheets]

class SheetBuilder(object):
    def __init__(self, idx, w, h):
        self.idx = idx
        self.image = Image.new('RGBA', (w * TILE_SIZE, h * TILE_SIZE))
        self.depthmap = Image.new('RGBA', (w * TILE_SIZE, h * TILE_SIZE))
        self.in_use = 0
        self.stride = h
        self.area = w * h

    def place(self, w, h):
        base_mask = (1 << w) - 1
        mask = 0
        for i in range(h):
            mask |= base_mask << (i * self.stride)

        i = 0
        in_use = self.in_use
        while in_use != 0 and i < self.area:
            if mask & in_use == 0:
                break
            i += 1
            in_use >>= 1

        if i < self.area:
            self.in_use |= mask << i
            return (i % self.stride, i // self.stride)
        else:
            return None


# JSON output

def build_client_json(structures):
    def convert(s):
        dct = {
                'size': s.size,
                'shape': [SHAPE_ID[x] for x in s.shape],
                'sheet': s.sheet_idx,
                'offset': s.offset,
                'display_size': s.image.size,
                'layer': s.layer,
                }
        if s.light_color is not None:
            dct.update(
                light_pos=s.light_pos,
                light_color=s.light_color,
                light_radius=s.light_radius)
        return dct

    return list(convert(s) for s in structures)

def build_server_json(structures):
    def convert(s):
        return {
                'name': s.name,
                'size': s.size,
                'shape': [SHAPE_ID[x] for x in s.shape],
                'layer': s.layer,
                }

    return list(convert(s) for s in structures)
