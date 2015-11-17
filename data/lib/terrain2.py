from outpost_data.core.consts import *
from outpost_data.core.builder2 import *
from outpost_data.core import image2


def _dict_blocks(dct, basename, img, shape='empty', base_img=None):
    parts = img.chop(dct)
    if base_img is not None:
        parts = {k: base_img.stack((v,)) for k,v in parts.items()}

    b = BLOCK.prefixed(basename)
    b.new(parts.keys()) \
            .bottom(parts) \
            .shape(shape)
    return b

def terrain_blocks(basename, img, **kwargs):
    return _dict_blocks(TERRAIN_PARTS2, basename, img, **kwargs)

def terrain_cross_blocks(basename, img, **kwargs):
    return _dict_blocks(TERRAIN_CROSS_PARTS2, basename, img, **kwargs)


INTERIOR_PART_NAMES = (
        ('nw/full',     'ne/full',      None,           None),
        ('sw/full',     'se/full',      None,           None),
        ('nw/outer',    'ne/horiz',     'nw/horiz',     'ne/outer'),
        ('sw/vert',     'se/inner',     'sw/inner',     'se/vert'),
        ('nw/vert',     'ne/inner',     'nw/inner',     'ne/vert'),
        ('sw/outer',    'se/horiz',     'sw/horiz',     'se/outer'),
        )
INTERIOR_PARTS = {n: (x, y)
        for y, ns in enumerate(INTERIOR_PART_NAMES)
        for x, n in enumerate(ns)}

def interior_blocks(basename, img, shape='empty'):
    parts = img.chop(INTERIOR_PARTS, unit=TILE_SIZE // 2)

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
    def add(base, parts_str):
        for k, v in variations(base, parts_str.split()):
            PART_MAP_FULL[k] = v

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

    full_parts = {}
    HALF = TILE_SIZE // 2
    for k, (nw, sw, se, ne) in PART_MAP_FULL.items():
        full_parts[k] = image2.stack((
                parts['nw/' + nw].pad(TILE_SIZE, offset=(0, 0)),
                parts['sw/' + sw].pad(TILE_SIZE, offset=(0, HALF)),
                parts['se/' + se].pad(TILE_SIZE, offset=(HALF, HALF)),
                parts['ne/' + ne].pad(TILE_SIZE, offset=(HALF, 0)),
                )).with_unit(TILE_SIZE)

    b = BLOCK.prefixed(basename) \
            .shape(shape)
    b.new(full_parts.keys()) \
            .bottom(full_parts)
    return b
