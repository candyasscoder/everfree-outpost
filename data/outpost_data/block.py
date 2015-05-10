from PIL import Image

from outpost_data.consts import *
from outpost_data.util import err


class BlockDef(object):
    def __init__(self, name, shape, tiles):
        self.name = name
        self.shape = shape
        self.tile_names = tiles

        self.light_color = None
        self.light_radius = None

        self.id = None
        self.tile_ids = None

    def set_light(self, color, radius):
        self.light_color = color
        self.light_radius = radius


def resolve_tile_ids(blocks, tile_id_map):
    for b in blocks:
        b.tile_ids = {}
        for side, name in b.tile_names.items():
            if name is None:
                continue

            tile_id = tile_id_map.get(name)
            if tile_id is None:
                err('block %r, side %r: no such tile: %r' % (b.name, side, name))
                continue

            b.tile_ids[side] = tile_id


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
