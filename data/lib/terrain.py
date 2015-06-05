from outpost_data.builder import *
from outpost_data.consts import *
from outpost_data.util import chop_terrain, chop_image_named

from PIL import Image


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


def mk_terrain_interior(img, basename, **kwargs):
    PART_NAMES_HALF = (
            ('nw/full',     'ne/full',      None,           None),
            ('sw/full',     'se/full',      None,           None),
            ('nw/outer',    'ne/horiz',     'nw/horiz',     'ne/outer'),
            ('sw/vert',     'se/inner',     'sw/inner',     'se/vert'),
            ('nw/vert',     'ne/inner',     'nw/inner',     'ne/vert'),
            ('sw/outer',    'se/horiz',     'sw/horiz',     'se/outer'),
            )

    parts_half = chop_image_named(img, PART_NAMES_HALF, TILE_SIZE // 2)

    def variations(k, vs):
        f = lambda x: (('0', 'inner'), ('1', 'full')) if x == '*' else (('', x),)

        for ak, av in f(vs[0]):
            for bk, bv in f(vs[1]):
                for ck, cv in f(vs[2]):
                    for dk, dv in f(vs[3]):
                        extra_k = ak + bk + ck + dk
                        local_k = k + '/' + extra_k if extra_k else k
                        yield (local_k, (av, bv, cv, dv))


    # Build up the map that describes how to build each tile from parts.
    PART_MAP_FULL = {}
    PART_ORDER = []
    def add(base, parts_str):
        for k, v in variations(base, parts_str.split()):
            PART_MAP_FULL[k] = v
            PART_ORDER.append(k)

    # Naming convention: <edges>/<corners>
    #
    # <edges> is [n][s][w][e] depending on which edges are filled.
    #
    # <corners> is [01] for each corner with both connected edges filled
    # (order: nw, sw, se, ne).  The digit is 0 if the subtile in that position
    # is 'inner' (for situations where there is not a tile of the same group in
    # that direction), and 1 if the subtile is 'full' (there is a tile).
    #
    # If <corners> is empty, then the slash is omitted.

    #               'NW    SW    SE    NE   '
    add('spot',     'outer outer outer outer')

    add('n',        'vert  outer outer vert ')
    add('w',        'horiz horiz outer outer')
    add('s',        'outer vert  vert  outer')
    add('e',        'outer outer horiz horiz')

    add('nw',       '*     horiz outer vert ')
    add('sw',       'horiz *     vert  outer')
    add('se',       'outer vert  *     horiz')
    add('ne',       'vert  outer horiz *    ')
    add('ns',       'vert  vert  vert  vert ')
    add('we',       'horiz horiz horiz horiz')

    add('nsw',      '*     *     vert  vert ')
    add('swe',      'horiz *     *     horiz')
    add('nse',      'vert  vert  *     *    ')
    add('nwe',      '*     horiz horiz *    ')

    add('nswe',     '*     *     *     *    ')


    img = Image.new('RGBA', (TILE_SIZE, len(PART_MAP_FULL) * TILE_SIZE))
    HALF = TILE_SIZE // 2
    for i, k in enumerate(PART_ORDER):
        (nw, sw, se, ne) = PART_MAP_FULL[k]
        x = 0
        y = i * TILE_SIZE
        img.paste(parts_half['nw/' + nw], (x, y))
        img.paste(parts_half['sw/' + sw], (x, y + HALF))
        img.paste(parts_half['se/' + se], (x + HALF, y + HALF))
        img.paste(parts_half['ne/' + ne], (x + HALF, y))

    parts = {}
    for i, k in enumerate(PART_ORDER):
        x = 0
        y = i * TILE_SIZE
        parts[k] = img.crop((x, y, x + TILE_SIZE, y + TILE_SIZE))

    mk_floor_from_dict(basename, parts, **kwargs)
