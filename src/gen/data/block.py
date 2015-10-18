import itertools

from PIL import Image

from outpost_data.core import util
from outpost_data.core.consts import *
from outpost_data.core.util import err

from outpost_data.core.loader import TimeIt


class BlockDef(object):
    def __init__(self, name, shape, tiles):
        self.name = name
        self.shape = shape
        self.tiles = tiles

        self.light_color = None
        self.light_radius = None

        self.id = None
        self.tile_ids = None

    def set_light(self, color, radius):
        self.light_color = color
        self.light_radius = radius


def build_sheet(blocks):
    """Build a sprite sheet containing all the tile images for the provided
    blocks.  This also updates each block's `tile_ids` field with its index in
    the generated sheet."""

    # Assign an index to each tile image.  Force a blank tile at index 0.
    blank_tile = Image.new('RGBA', (TILE_SIZE, TILE_SIZE))
    block_gen = (i for b in blocks for i in b.tiles.values() if i is not None)
    img_list, idx_map = util.dedupe_images(itertools.chain((blank_tile,), block_gen))

    # Compute `tile_ids`.
    for b in blocks:
        b.tile_ids = dict((k, idx_map[id(v)]) for k,v in b.tiles.items() if v is not None)

    # Construct the sheet image.
    num_pages, offsets = util.pack_boxes_uniform(SHEET_SIZE, len(img_list))
    assert num_pages == 1, 'too many tile images to fit on one sheet'
    sheet, = util.build_sheets(img_list, offsets, 1, SHEET_SIZE, TILE_SIZE)

    return sheet


def build_client_json(blocks):
    def convert(b):
        dct = {
                'shape': SHAPE_ID[b.shape],
                }
        for k in BLOCK_SIDES:
            if k in b.tile_ids:
                dct[k] = b.tile_ids[k]
        if b.light_color is not None:
            red, green, blue = b.light_color
            dct.update(
                light_r=red,
                light_g=green,
                light_b=blue,
                light_radius=b.light_radius)
        return dct

    return list(convert(b) for b in blocks)

def build_server_json(blocks):
    def convert(b):
        return {
                'name': b.name,
                'shape': b.shape,
                }

    return list(convert(b) for b in blocks)
