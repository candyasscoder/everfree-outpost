from ..core.builder import *
from ..core.images import loader
from ..core import depthmap
from ..core.structure import Shape
from ..core.util import extract, stack

from .lib.items import *
from .lib.structures import *


def do_house_parts(basename, image, plane_image, door_image):
    house_parts = (
            (
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
            ),
            (
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
            ),
        )

    b = structure_builder()

    for i, row in enumerate(house_parts):
        for j, part_name in enumerate(row):
            name = basename + '/' + part_name
            b.merge(mk_solid_structure(
                name, image, (1, 1, 2), base=(j, i * 3), plane_image=plane_image))

    open_door_shape_arr = [
            'solid', 'floor', 'solid',
            'solid', 'empty', 'solid',
            ]
    open_door_shape = Shape(3, 1, 2, open_door_shape_arr)

    closed_door_shape_arr = [
            'solid', 'solid', 'solid',
            'solid', 'solid', 'solid',
            ]
    closed_door_shape = Shape(3, 1, 2, closed_door_shape_arr)

    w = 3 * TILE_SIZE
    h = 3 * TILE_SIZE

    x = 10 * TILE_SIZE
    y = 0
    open_door_img = image.crop((x, y, x + w, y + h))
    closed_door_img = stack(open_door_img, door_image)
    door_depth = depthmap.from_planemap(plane_image.crop((x, y, x + w, y + h)))
    b.create(basename + '/door/in/closed', closed_door_img, door_depth, closed_door_shape, 1)
    b.create(basename + '/door/in/open', open_door_img, door_depth, open_door_shape, 1)

    x = 13 * TILE_SIZE
    y = 0
    open_door_img = image.crop((x, y, x + w, y + h))
    closed_door_img = stack(open_door_img, door_image)
    door_depth = depthmap.from_planemap(plane_image.crop((x, y, x + w, y + h)))
    b.create(basename + '/door/out/closed', closed_door_img, door_depth, closed_door_shape, 1)
    b.create(basename + '/door/out/open', open_door_img, door_depth, open_door_shape, 1)

    return b

def init():
    structures = loader('structures')

    house = do_house_parts('house_wall',
            structures('house.png'), structures('house-planemap.png'),
            structures('door.png'))

    i = item_builder()
    i.merge(mk_structure_item(house['house_wall/edge/horiz/in'],
        'house_wall/side', 'House Side', (0, 0)))
    i.merge(mk_structure_item(house['house_wall/corner/nw/in'],
        'house_wall/corner', 'House Corner', (0, 0)))
    i.merge(mk_structure_item(house['house_wall/tee/e/in'],
        'house_wall/tee', 'House Tee', (0, 0)))
    i.merge(mk_structure_item(house['house_wall/cross/in_in'],
        'house_wall/cross', 'House Cross', (0, 0)))
    i.recipe('anvil', {'wood': 5})

    mk_structure_item(house['house_wall/door/in/closed'], 'house_door', 'House Door') \
            .recipe('anvil', {'wood': 15})

    floor = mk_terrain_structures('wood_floor', structures('wood-floor.png'))
    mk_structure_item(floor['wood_floor/center/v0'], 'house_floor', 'House Floor') \
            .recipe('anvil', {'wood': 5})
