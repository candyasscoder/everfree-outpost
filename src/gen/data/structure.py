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


class StaticAnimDef(object):
    """An animation for a static element (structure or block).  This class is
    distinct from AnimationDef (sprite animations) because the requirements are
    not actually very similar."""
    def __init__(self, sheet, length, framerate):
        self.length = length
        self.framerate = framerate
        self.sheet = sheet

        w, h = sheet.size
        self.size = (w // length, h)


class StructureDef(object):
    def __init__(self, name, image, depthmap, shape, layer):
        self.name = name
        if isinstance(image, StaticAnimDef):
            self.anim = image
            # Also provide a reasonable image, for mk_structure_item and such.
            w, h = self.anim.size
            self.image = self.anim.sheet.crop((0, 0, w, h))
        else:
            self.image = image
            self.anim = None
        self.depthmap = depthmap
        self.size = shape.size
        self.shape = shape.shape_array
        self.layer = layer

        # Both StaticAnimDef and PIL.Image.Image have a `size` field with the
        # same layout.
        assert image.size == depthmap.size
        assert image.size[0] % TILE_SIZE == 0
        assert image.size[1] % TILE_SIZE == 0

        self.light_pos = None
        self.light_color = None
        self.light_radius = None

        self.id = None
        self.sheet_idx = None
        self.offset = None

    def get_display_px(self):
        """Get the size of the structure on the screen."""
        if self.anim is None:
            return self.image.size
        else:
            return self.anim.size

    def get_sheet_px(self):
        """Get the size of the structure in the sprite sheet.  This may be
        larger than the size on screen if the structure has multiple animation
        frames."""
        if self.anim is None:
            return self.image.size
        else:
            return self.anim.sheet.size

    def get_display_size(self):
        w, h = self.get_display_px()
        return (w // TILE_SIZE, h // TILE_SIZE)

    def get_sheet_size(self):
        w, h = self.get_sheet_px()
        return (w // TILE_SIZE, h // TILE_SIZE)

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
    boxes = [s.get_sheet_size() for s in structures]
    num_sheets, offsets = pack_boxes(SHEET_SIZE, boxes)


    images = [Image.new('RGBA', (SHEET_PX, SHEET_PX))]
    depthmaps = [Image.new('RGBA', (SHEET_PX, SHEET_PX))]

    for s, (j, offset) in zip(structures, offsets):
        size = s.get_sheet_size()

        x, y = offset
        x *= TILE_SIZE
        y *= TILE_SIZE

        px_w, px_h = s.image.size
        if px_h % TILE_SIZE != 0:
            y += 32 - px_h % TILE_SIZE

        if s.anim is None:
            images[j].paste(s.image, (x, y))
        else:
            images[j].paste(s.anim.sheet, (x, y))
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
                'offset': [x * TILE_SIZE for x in s.offset],
                'display_size': s.get_display_size(),
                'layer': s.layer,
                }
        if s.anim is not None:
            dct.update(
                anim_length=s.anim.length,
                anim_rate=s.anim.framerate)
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
