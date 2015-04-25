import os

from PIL import Image

from outpost_data.consts import *
from outpost_data import depthmap
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
            depth = depthmap.flat(TILE_SIZE, TILE_SIZE)
            name = basename + '/' + part_name
            s.append(StructureDef(name, tile, depth, floor(1, 1, 1), 0))

def solid_structure(s, name, image, size, base=(0, 0), display_size=None,
        plane_image=None, layer=1):
    base_x, base_y = base
    x = base_x * TILE_SIZE
    y = base_y * TILE_SIZE

    size_x, size_y, size_z = size
    if display_size is not None:
        display_size_x, display_size_y = display_size
        w = display_size_x * TILE_SIZE
        h = display_size_y * TILE_SIZE
    else:
        w = size_x * TILE_SIZE
        h = (size_y + size_z) * TILE_SIZE

    struct_img = image.crop((x, y, x + w, y + h))
    if plane_image is None:
        depth = depthmap.solid(size_x * TILE_SIZE, size_y * TILE_SIZE, size_z * TILE_SIZE)
        # Cut a w*h sized section from the bottom.
        depth_height = depth.size[1]
        depth = depth.crop((0, depth_height - h, w, depth_height))
    else:
        depth = depthmap.from_planemap(plane_image.crop((x, y, x + w, y + h)))
    s.append(StructureDef(name, struct_img, depth, solid(*size), layer))


def solid_small(s, name, image, base=(0, 0)):
    solid_structure(s, name, image, (1, 1, 1), base=base, display_size=(1, 1))

def do_tree(s, image, plane_image):
    tree_shape_arr = [
            'floor', 'solid', 'solid', 'floor',
            'floor', 'solid', 'solid', 'floor',

            'empty', 'solid', 'solid', 'empty',
            'empty', 'solid', 'solid', 'empty',

            'empty', 'solid', 'solid', 'empty',
            'empty', 'solid', 'solid', 'empty',
            ]
    tree_shape = Shape(4, 2, 3, tree_shape_arr)
    stump_shape = Shape(4, 2, 1, tree_shape_arr[:8])

    tree_bounds = (0, 0, 4 * TILE_SIZE, 5 * TILE_SIZE)
    stump_bounds = (0, 5 * TILE_SIZE, 4 * TILE_SIZE, 8 * TILE_SIZE)

    s.append(StructureDef(
        'tree',
        image.crop(tree_bounds),
        depthmap.from_planemap(plane_image.crop(tree_bounds)),
        tree_shape,
        1))

    s.append(StructureDef(
        'stump',
        image.crop(stump_bounds),
        depthmap.from_planemap(plane_image.crop(stump_bounds)),
        stump_shape,
        1))

def do_house_parts(s, basename, image, plane_image):
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
            solid_structure(s, name, image, (1, 1, 2), base=(j, i * 3), plane_image=plane_image)

    door_shape_arr = [
            'solid', 'floor', 'solid',
            'solid', 'empty', 'solid',
            ]
    door_shape = Shape(3, 1, 2, door_shape_arr)

    w = 3 * TILE_SIZE
    h = 3 * TILE_SIZE

    x = 10 * TILE_SIZE
    y = 0
    door_img = image.crop((x, y, x + w, y + h))
    door_depth = depthmap.from_planemap(plane_image.crop((x, y, x + w, y + h)))
    s.append(StructureDef(basename + '/door/in', door_img, door_depth, door_shape, 1))

    x = 13 * TILE_SIZE
    y = 0
    door_img = image.crop((x, y, x + w, y + h))
    door_depth = depthmap.from_planemap(plane_image.crop((x, y, x + w, y + h)))
    s.append(StructureDef(basename + '/door/out', door_img, door_depth, door_shape, 1))

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

def do_statue(s, basename, image):
    parts = [image.crop((x * 64, 0, (x + 1) * 64, 96)) for x in (0, 1, 2)]
    solid_structure(s, basename + '/n', parts[0], (2, 1, 2))
    solid_structure(s, basename + '/s', parts[1], (2, 1, 2))
    solid_structure(s, basename + '/e', parts[2], (2, 1, 2))
    solid_structure(s, basename + '/w', parts[2].transpose(Image.FLIP_LEFT_RIGHT), (2, 1, 2))


def get_structures(asset_path):
    path = os.path.join(asset_path, 'structures')
    img = lambda name: I.load(os.path.join(path, name))

    s = []

    terrain_floor(s, 'wood_floor', img('wood-floor.png'))
    terrain_floor(s, 'road', img('road.png'))

    do_tree(s, img('tree.png'), img('tree-planemap.png'))
    solid_structure(s, 'rock', img('rock.png'), (2, 1, 1), (0, 0))

    solid_small(s, 'anvil', img('anvil.png'))
    solid_small(s, 'chest', img('chest.png'))
    solid_small(s, 'teleporter', img('crystal-formation.png'))
    solid_small(s, 'dungeon_entrance', img('crystal-formation-red.png'))
    solid_structure(s, 'ward', img('crystal-ward.png'), (1, 1, 1))

    image = img('furniture.png')
    plane = img('furniture-planemap.png')
    solid_structure(s, 'bed', image, (2, 2, 1), (0, 0), plane_image=plane)
    solid_structure(s, 'table', image, (2, 2, 1), (2, 0), plane_image=plane)

    solid_structure(s, 'cabinets', image, (1, 1, 2), (4, 0), plane_image=plane, layer=2)
    solid_structure(s, 'bookshelf/0', image, (1, 1, 2), (5, 0), plane_image=plane, layer=2)
    solid_structure(s, 'bookshelf/1', image, (1, 1, 2), (6, 0), plane_image=plane, layer=2)
    solid_structure(s, 'bookshelf/2', image, (1, 1, 2), (7, 0), plane_image=plane, layer=2)

    do_house_parts(s, 'house_wall', img('house.png'), img('house-planemap.png'))
    do_fence_parts(s, 'fence', img('fence.png'))
    do_statue(s, 'statue', img('statue.png'))

    return s
