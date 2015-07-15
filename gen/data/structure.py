from PIL import Image

from .consts import *
from .util import pack_boxes


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
    boxes = [s.get_display_size() for s in structures]
    num_sheets, offsets = pack_boxes(SHEET_SIZE, boxes)


    images = [Image.new('RGBA', (SHEET_PX, SHEET_PX))]
    depthmaps = [Image.new('RGBA', (SHEET_PX, SHEET_PX))]

    for s, (j, offset) in zip(structures, offsets):
        size = s.get_display_size()

        x, y = offset
        x *= TILE_SIZE
        y *= TILE_SIZE

        px_w, px_h = s.image.size
        if px_h % TILE_SIZE != 0:
            y += 32 - px_h % TILE_SIZE

        images[j].paste(s.image, (x, y))
        depthmaps[j].paste(s.depthmap, (x, y))

        s.sheet_idx = j
        s.offset = offset

    return zip(images, depthmaps)


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
