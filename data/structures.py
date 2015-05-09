import os

from PIL import Image

from outpost_data.builder import *
from outpost_data.consts import *
from outpost_data import depthmap
import outpost_data.images as I
from outpost_data.structure import Shape, floor, solid
from outpost_data.util import *

def mk_terrain_floor(basename, image):
    structs = {}
    depth = depthmap.flat(TILE_SIZE, TILE_SIZE)
    shape = floor(1, 1, 1)

    for k, tile in chop_terrain(image).items():
        name = basename + '/' + k
        structs[k] = mk_structure(name, tile, depth, shape, 0)

    return structs

def mk_solid_structure(name, image, size, base=(0, 0), display_size=None,
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

    return mk_structure(name, struct_img, depth, solid(*size), layer)


def mk_solid_small(name, image, **kwargs):
    """Make a small, solid structure: a solid structure with size 1x1x1, but
    only a 1x1 tile (for the front, nothing on the top)."""
    return mk_solid_structure(name, image, (1, 1, 1), display_size=(1, 1), **kwargs)

def do_tree(image, plane_image):
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

    mk_structure(
        'tree',
        image.crop(tree_bounds),
        depthmap.from_planemap(plane_image.crop(tree_bounds)),
        tree_shape,
        1)

    mk_structure(
        'stump',
        image.crop(stump_bounds),
        depthmap.from_planemap(plane_image.crop(stump_bounds)),
        stump_shape,
        1)

def do_house_parts(basename, image, plane_image):
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
            mk_solid_structure(name, image, (1, 1, 2), base=(j, i * 3), plane_image=plane_image)

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
    mk_structure(basename + '/door/in', door_img, door_depth, door_shape, 1)

    x = 13 * TILE_SIZE
    y = 0
    door_img = image.crop((x, y, x + w, y + h))
    door_depth = depthmap.from_planemap(plane_image.crop((x, y, x + w, y + h)))
    mk_structure(basename + '/door/out', door_img, door_depth, door_shape, 1)

def do_fence_parts(basename, image):
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
            mk_solid_small(name, image, base=(j, i))

def do_statue(basename, image):
    parts = [image.crop((x * 64, 0, (x + 1) * 64, 96)) for x in (0, 1, 2)]
    mk_solid_structure(basename + '/n', parts[0], (2, 1, 2))
    mk_solid_structure(basename + '/s', parts[1], (2, 1, 2))
    mk_solid_structure(basename + '/e', parts[2], (2, 1, 2))
    mk_solid_structure(basename + '/w', parts[2].transpose(Image.FLIP_LEFT_RIGHT), (2, 1, 2))


def init(asset_path):
    path = os.path.join(asset_path, 'structures')
    img = lambda name: I.load(os.path.join(path, name))

    mk_terrain_floor('wood_floor', img('wood-floor.png'))
    mk_terrain_floor('road', img('road.png'))

    do_tree(img('tree.png'), img('tree-planemap.png'))
    mk_solid_structure('rock', img('rock.png'), (2, 1, 1), (0, 0))

    mk_solid_small('anvil', img('anvil.png'))
    mk_solid_small('chest', img('chest.png'))
    mk_solid_small('teleporter', img('crystal-formation.png')) \
            .set_light((16, 16, 16), (48, 48, 96), 50)
    mk_solid_small('dungeon_entrance', img('crystal-formation-red.png')) \
            .set_light((16, 16, 16), (96, 48, 48), 50)
    mk_solid_small('dungeon_exit', img('crystal-formation-red.png')) \
            .set_light((16, 16, 16), (96, 48, 48), 50)
    mk_solid_structure('ward', img('crystal-ward.png'), (1, 1, 1)) \
            .set_light((16, 16, 48), (48, 48, 96), 50)
    mk_solid_small('script_trigger', img('crystal-formation-green.png')) \
            .set_light((16, 16, 16), (48, 96, 48), 50)
    mk_solid_structure('trophy', img('trophy.png'), (1, 1, 1))
    mk_solid_structure('fountain', img('fountain.png'), (2, 2, 1))
    mk_solid_structure('torch', img('torch.png'), (1, 1, 1)) \
            .set_light((16, 16, 32), (255, 230, 200), 300)

    image = img('furniture.png')
    plane = img('furniture-planemap.png')
    mk_solid_structure('bed', image, (2, 2, 1), (0, 0), plane_image=plane)
    mk_solid_structure('table', image, (2, 2, 1), (2, 0), plane_image=plane)

    mk_solid_structure('cabinets', image, (1, 1, 2), (4, 0), plane_image=plane, layer=2)
    mk_solid_structure('bookshelf/0', image, (1, 1, 2), (5, 0), plane_image=plane, layer=2)
    mk_solid_structure('bookshelf/1', image, (1, 1, 2), (6, 0), plane_image=plane, layer=2)
    mk_solid_structure('bookshelf/2', image, (1, 1, 2), (7, 0), plane_image=plane, layer=2)

    do_house_parts('house_wall', img('house.png'), img('house-planemap.png'))
    do_fence_parts('fence', img('fence.png'))
    do_statue('statue', img('statue.png'))
