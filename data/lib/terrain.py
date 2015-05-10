from outpost_data.builder import *
from outpost_data.consts import *
from outpost_data.util import chop_terrain

def mk_floor_from_dict(basename, dct, shape='empty', base_img=None):
    b = block_builder()
    for part_name, part_img in dct.items():
        if base_img is not None:
            img = base_img.copy()
            img.paste(part_img, (0, 0), part_img)
        else:
            img = part_img

        b.create('%s/%s' % (basename, part_name), shape, {'bottom': img})
    return b

def mk_floor_blocks(img, basename, **kwargs):
    dct = chop_terrain(img)
    dct['center'] = dct['center/v0']
    return mk_floor_from_dict(basename, dct, **kwargs)

def mk_floor_cross(img, basename, **kwargs):
    dct = {
            'cross/nw': img.crop((0, 0, TILE_SIZE, TILE_SIZE)),
            'cross/ne': img.crop((0, TILE_SIZE, TILE_SIZE, 2 * TILE_SIZE)),
            }
    return mk_floor_from_dict(basename, dct, **kwargs)
