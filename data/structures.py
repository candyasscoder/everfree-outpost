import os

from outpost_data.consts import *
import outpost_data.images as I
from outpost_data.structure import StructureDef, Shape, floor, solid

def terrain_floor(s, basename, image):
    parts = [
            ['spot/large',      'corner/inner/se',  'corner/inner/sw'],
            ['spot/small',      'corner/inner/ne',  'corner/inner/nw'],
            ['corner/outer/nw', 'edge/n',           'corner/outer/ne'],
            ['edge/w',          'center/v0',        'edge/e'],
            ['corner/outer/sw', 'edge/s',           'corner/outer/se'],
            ['center/v1',       'center/v2',        'center/v3'],
            ]

    for i, row in enumerate(parts):
        for j, part_name in enumerate(row):
            x = j * TILE_SIZE
            y = i * TILE_SIZE
            tile = image.crop((x, y, x + TILE_SIZE, y + TILE_SIZE))
            name = basename + '/' + part_name
            s.append(StructureDef(name, tile, floor(1, 1, 1), 0))

def solid_structure(s, name, image, size, base=(0, 0), display_size=None):
    base_x, base_y = base
    x = base_x * TILE_SIZE
    y = base_y * TILE_SIZE

    if display_size is not None:
        display_size_x, display_size_y = display_size
        w = display_size_x * TILE_SIZE
        h = display_size_y * TILE_SIZE
    else:
        size_x, size_y, size_z = size
        w = size_x * TILE_SIZE
        h = (size_y + size_z) * TILE_SIZE

    struct_img = image.crop((x, y, x + w, y + h))
    s.append(StructureDef(name, struct_img, solid(*size), 1))


def solid_small(s, name, image, base=(0, 0)):
    solid_structure(s, name, image, (1, 1, 1), base=base, display_size=(1, 1))

def do_house_parts(s, basename, image):
    house_parts = [
            [
                'corner/nw/in',
                'edge/horiz/in',
                'corner/ne/in',
                'corner/sw/out',
                'edge/horiz/out',
                'corner/se/out',
                'edge/vert',
                'tee/n/in',
                'tee/n/out',
                # Doors are handled separately.
            ],
            [
                'corner/nw/out',
                'corner/ne/out',
                'corner/sw/in',
                'corner/se/in',
                'tee/e/out',
                'tee/w/out',
                'tee/e/in',
                'tee/w/in',
                'tee/s/out_in',
                'tee/s/in_out',
                'tee/s/in_in',
                'tee/s/out_out',
                'cross/out_in',
                'cross/in_out',
                'cross/in_in',
                'cross/out_out',
            ],
        ]

    for i, row in enumerate(house_parts):
        for j, part_name in enumerate(row):
            name = basename + '/' + part_name
            solid_structure(s, name, image, (1, 2, 1), base=(j, i * 3))

    door_shape_arr = [
            'solid', 'floor', 'solid',
            'solid', 'empty', 'solid',
            ]
    door_shape = Shape(3, 1, 2, door_shape_arr)

    w = 3 * TILE_SIZE
    h = 3 * TILE_SIZE

    x = 10 * TILE_SIZE
    y = 0
    s.append(StructureDef(basename + '/door/in', image.crop((x, y, x + w, y + h)), door_shape, 1))

    x = 13 * TILE_SIZE
    y = 0
    s.append(StructureDef(basename + '/door/out', image.crop((x, y, x + w, y + h)), door_shape, 1))

def do_fence_parts(s, basename, image):
    fence_parts = [
            ['end/w',           'edge/horiz',       'end/e'],
            ['end/s',           'edge/vert',        'end/n'],
            ['corner/nw',       'tee/s',            'corner/ne'],
            ['tee/e',           'cross',            'tee/w'],
            ['corner/sw',       'tee/n',            'corner/se'],
            ['end/fancy/e',     'gate',             'end/fancy/w'],
            ]

    for i, row in enumerate(fence_parts):
        for j, part_name in enumerate(row):
            name = basename + '/' + part_name
            solid_small(s, name, image, (j, i))


def get_structures(asset_path):
    path = os.path.join(asset_path, 'structures')
    img = lambda name: I.load(os.path.join(path, name))

    s = []

    terrain_floor(s, 'wood_floor', img('wood-floor.png'))
    terrain_floor(s, 'road', img('road.png'))

    solid_structure(s, 'tree', img('tree.png'), (4, 2, 3), (0, 0))
    solid_structure(s, 'stump', img('tree.png'), (4, 2, 1), (0, 5))

    solid_small(s, 'anvil', img('anvil.png'))
    solid_small(s, 'chest', img('chest.png'))
    solid_small(s, 'teleporter', img('crystal-formation.png'))
    solid_structure(s, 'ward', img('crystal-ward.png'), (1, 1, 1))

    solid_structure(s, 'bed', img('furniture.png'), (2, 2, 1), (0, 0))
    solid_structure(s, 'table', img('furniture.png'), (2, 2, 1), (2, 0))
    solid_structure(s, 'cabinets', img('furniture.png'), (1, 2, 1), (4, 0))
    solid_structure(s, 'bookshelf/0', img('furniture.png'), (1, 2, 1), (5, 0))
    solid_structure(s, 'bookshelf/1', img('furniture.png'), (1, 2, 1), (6, 0))
    solid_structure(s, 'bookshelf/2', img('furniture.png'), (1, 2, 1), (7, 0))

    do_house_parts(s, 'house_walls', img('house.png'))
    do_fence_parts(s, 'fence', img('fence.png'))

    return s
