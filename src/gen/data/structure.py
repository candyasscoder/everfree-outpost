from PIL import Image, ImageChops

from .consts import *
from .util import pack_boxes, err


class Shape(object):
    def __init__(self, x, y, z, arr):
        self.size = (x, y, z)
        self.shape_array = arr

def empty(x, y, z):
    arr = ['empty'] * (x * y * z)
    return Shape(x, y, z, arr)

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
    def __init__(self, frames, framerate, oneshot=False):
        self.length = len(frames)
        self.framerate = framerate
        self.oneshot = oneshot
        self.frames = frames
        self.size = frames[0].size

        # Set by `self.process_frames()`
        self.static_base = None
        self.anim_offset = None
        self.anim_size = None
        self.anim_sheet = None
        self.process_frames()

        # Set by `build_anim_sheets`
        self.sheet_idx = None
        self.offset = None

    def process_frames(self):
        assert all(f.size == self.frames[0].size for f in self.frames), \
                'all animation frames must be the same size'

        w, h = self.frames[0].size
        first = self.frames[0].getdata()
        px_size = len(first) // (w * h)

        base = self.frames[0].convert('RGBA')

        min_x, min_y = w, h
        max_x, max_y = 0, 0
        for f in self.frames[1:]:
            data = f.getdata()
            for i in range(w * h):
                if data[i] != first[i]:
                    i //= px_size
                    x = i % w
                    y = i // w
                    min_x = min(min_x, x)
                    min_y = min(min_y, y)
                    max_x = max(max_x, x + 1)
                    max_y = max(max_y, y + 1)

                    base.paste((0, 0, 0, 0), (x, y, x + 1, y + 1))

        self.static_base = base
        self.anim_offset = (min_x, min_y)
        self.anim_size = (max_x - min_x, max_y - min_y)
        
        sheet = Image.new('RGBA', (len(self.frames) * (max_x - min_x), max_y - min_y))
        for i, f in enumerate(self.frames):
            tmp = ImageChops.subtract(f, base)
            tmp = tmp.crop((min_x, min_y, max_x, max_y))
            sheet.paste(tmp, ((max_x - min_x) * i, 0))

        self.anim_sheet = sheet



class StructureDef(object):
    def __init__(self, name, image, model, shape, layer):
        self.name = name
        if isinstance(image, StaticAnimDef):
            self.anim = image
            self.image = self.anim.static_base
        else:
            self.image = image
            self.anim = None
        self.model_name = model
        self.size = shape.size
        self.shape = shape.shape_array
        self.layer = layer

        # Both StaticAnimDef and PIL.Image.Image have a `size` field with the
        # same layout.
        assert image.size[0] % TILE_SIZE == 0
        assert image.size[1] % TILE_SIZE == 0

        self.light_pos = None
        self.light_color = None
        self.light_radius = None

        self.id = None
        self.sheet_idx = None
        self.offset = None
        self.model_offset = None
        self.model_length = None

    def get_display_px(self):
        """Get the size of the structure on the screen."""
        return self.image.size

    def get_sheet_px(self):
        """Get the size of the structure in the sprite sheet.  Currently this
        is the same as `get_display_px` because animated structures place only
        the static parts in the main structure sheet."""
        return self.image.size

    def get_display_size(self):
        w, h = self.get_display_px()
        return (w // TILE_SIZE, h // TILE_SIZE)

    def get_sheet_size(self):
        w, h = self.get_sheet_px()
        return (w // TILE_SIZE, h // TILE_SIZE)

    def get_anim_sheet_px(self):
        return self.anim.anim_sheet.size

    def get_anim_sheet_size(self):
        w, h = self.get_anim_sheet_px()
        return ((w + TILE_SIZE - 1) // TILE_SIZE,
                (h + TILE_SIZE - 1) // TILE_SIZE)

    def set_light(self, pos, color, radius):
        self.light_pos = pos
        self.light_color = color
        self.light_radius = radius

    def get_flags(self):
        if self.image.mode == 'RGBA':
            hist = self.image.histogram()
            a_hist = hist[256 * 3 : 256 * 4]
            # Is there any pixel whose alpha value is not 0 or 255?
            has_shadow = any(count > 0 for count in a_hist[1:-1])
        else:
            has_shadow = false

        has_anim = self.anim is not None
        has_light = self.light_radius is not None

        return ((int(has_shadow) << 0) |
                (int(has_anim) << 1) |
                (int(has_light) << 2))


def resolve_model_offsets(structures, model_offset_map):
    for s in structures:
        offset_length = model_offset_map.get(s.model_name)
        if offset_length is None:
            err('structure %r: no such model: %r' % (s.name, s.model_name))
            continue
        s.model_offset, s.model_length = offset_length

# Sprite sheets

def build_sheets(structures):
    '''Build sprite sheet(s) containing all the images for the provided
    structures.  This also updates each structure's `sheet_idx` and `offset`
    field with its position in the generated sheet(s).
    '''
    boxes = [s.get_sheet_size() for s in structures]
    num_sheets, offsets = pack_boxes(SHEET_SIZE, boxes)


    images = [Image.new('RGBA', (SHEET_PX, SHEET_PX))]

    for s, (j, offset) in zip(structures, offsets):
        x, y = offset
        x *= TILE_SIZE
        y *= TILE_SIZE

        px_w, px_h = s.image.size
        if px_h % TILE_SIZE != 0:
            y += 32 - px_h % TILE_SIZE

        images[j].paste(s.image, (x, y))

        s.sheet_idx = j
        s.offset = offset

    return images

def build_anim_sheets(structures):
    '''Build sprite sheet(s) containing the animated parts of each structure.'''
    anim_structures = [s for s in structures if s.anim is not None]
    boxes = [s.get_anim_sheet_size() for s in anim_structures]
    num_sheets, offsets = pack_boxes(SHEET_SIZE, boxes)


    images = [Image.new('RGBA', (SHEET_PX, SHEET_PX))]

    for s, (j, offset) in zip(anim_structures, offsets):
        x, y = offset
        x *= TILE_SIZE
        y *= TILE_SIZE

        images[j].paste(s.anim.anim_sheet, (x, y))

        s.anim.sheet_idx = j
        # Give the offset in pixels, since the anim size is also in pixels.
        ox, oy = offset
        s.anim.offset = (ox * TILE_SIZE, oy * TILE_SIZE)

    return images


# JSON output

def build_client_json(structures):
    def convert(s):
        dct = {
                'size': s.size,
                'shape': [SHAPE_ID[x] for x in s.shape],
                'sheet': s.sheet_idx,
                'offset': [x * TILE_SIZE for x in s.offset],
                'display_size': s.get_display_px(),
                'layer': s.layer,
                }

        flags = s.get_flags()
        if flags != 0:
            dct.update(
                flags=flags)

        if s.anim is not None:
            dct.update(
                anim_length=s.anim.length,
                anim_rate=s.anim.framerate,
                anim_oneshot=s.anim.oneshot,
                anim_pos=s.anim.anim_offset,
                anim_size=s.anim.anim_size,
                anim_offset=s.anim.offset,
                anim_sheet=s.anim.sheet_idx)

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
