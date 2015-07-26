from ..core.builder import *
from ..core.images import loader
from ..core import depthmap
from ..core.structure import Shape
from ..core.util import extract, stack

from .lib.items import *
from .lib.structures import *


def do_wall_parts(basename, image, plane_image, door_image=None):
    parts = (
            'corner/nw',
            'edge/horiz',
            'corner/ne',
            'corner/sw',
            '_/edge/horiz/copy',
            'corner/se',
            'edge/vert',
            'tee/e',
            'tee/w',
            'tee/n',
            'tee/s',
            'cross',
            # Doors are handled separately.
        )

    b = structure_builder()

    for j, part_name in enumerate(parts):
        name = basename + '/' + part_name
        b.merge(mk_solid_structure(name, image, (1, 1, 2), base=(j, 0), plane_image=plane_image))

    if door_image is not None:
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

        x = len(parts) * TILE_SIZE
        y = 0
        open_door_img = image.crop((x, y, x + w, y + h))
        closed_door_img = stack(open_door_img, door_image)
        door_depth = depthmap.from_planemap(plane_image.crop((x, y, x + w, y + h)))
        b.create(basename + '/door/closed', closed_door_img, door_depth, closed_door_shape, 1)
        b.create(basename + '/door/open', open_door_img, door_depth, open_door_shape, 1)

    return b

def init():
    structures = loader('structures')

    wall = do_wall_parts('wood_wall',
            structures('wood_wall.png'), structures('wood_wall-planemap.png'),
            door_image=structures('door.png'))

    mk_structure_item(wall['wood_wall/edge/horiz'], 'wood_wall', 'Wooden Wall', (0, 0)) \
        .recipe('anvil', {'wood': 5})

    mk_structure_item(wall['wood_wall/door/closed'], 'wood_door', 'Wooden Door') \
            .recipe('anvil', {'wood': 15})



    image = structures('stone-wall.png')
    planemap = structures('stone-wall-planemap.png')
    wall = do_wall_parts('stone_wall',
            structures('stone-wall.png'), structures('stone-wall-planemap.png'),
            door_image=structures('door.png'))
    mk_solid_structure('stone_wall/window/v0', image, (1, 1, 2), base=(15, 0),
            plane_image=planemap)
    mk_solid_structure('stone_wall/window/v1', image, (1, 1, 2), base=(16, 0),
            plane_image=planemap)

    mk_structure_item(wall['stone_wall/edge/horiz'], 'stone_wall', 'Stone Wall', (0, 0)) \
        .recipe('anvil', {'stone': 5})

    mk_structure_item(wall['stone_wall/door/closed'], 'stone_door', 'Stone Door') \
            .recipe('anvil', {'stone': 15})


    image = structures('ruined-wall.png')
    planemap = structures('stone-wall-planemap.png')
    wall = do_wall_parts('ruined_wall', image, planemap,
            door_image=structures('door.png'))
    mk_solid_structure('ruined_wall/window/v0', image, (1, 1, 2), base=(15, 0),
            plane_image=planemap)
    mk_solid_structure('ruined_wall/window/v1', image, (1, 1, 2), base=(16, 0),
            plane_image=planemap)

    mk_structure_item(wall['ruined_wall/edge/horiz'], 'ruined_wall', 'Ruined Wall', (0, 0)) \
        .recipe('anvil', {'stone': 5})

    mk_structure_item(wall['ruined_wall/door/closed'], 'ruined_door', 'Ruined Door') \
            .recipe('anvil', {'stone': 15})


    image = structures('cottage-wall.png')
    planemap = structures('cottage-wall-planemap.png')
    wall = do_wall_parts('cottage_wall',
            structures('cottage-wall.png'), structures('cottage-wall-planemap.png'),
            door_image=structures('door.png'))
    for i in range(2):
        mk_solid_structure('cottage_wall/window/v%d' % i, image, (1, 1, 2), base=(15 + i, 0),
                plane_image=planemap)
    for i in range(3):
        mk_solid_structure('cottage_wall/variant/v%d' % i, image, (1, 1, 2), base=(17 + i, 0),
                plane_image=planemap)

    mk_structure_item(wall['cottage_wall/edge/horiz'], 'cottage_wall', 'Cottage Wall', (0, 0)) \
        .recipe('anvil', {'wood': 5})

    mk_structure_item(wall['cottage_wall/door/closed'], 'cottage_door', 'Cottage Door') \
            .recipe('anvil', {'wood': 15})
