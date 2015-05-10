from outpost_data.builder import *
import outpost_data.images as I
from outpost_data import depthmap
from outpost_data.structure import Shape
from outpost_data.util import loader, extract

from lib.items import *
from lib.structures import *


def do_wall_parts(basename, image, plane_image, mk_door=False):
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

    if mk_door:
        door_shape_arr = [
                'solid', 'floor', 'solid',
                'solid', 'empty', 'solid',
                ]
        door_shape = Shape(3, 1, 2, door_shape_arr)

        w = 3 * TILE_SIZE
        h = 3 * TILE_SIZE

        x = len(parts) * TILE_SIZE
        y = 0
        door_img = image.crop((x, y, x + w, y + h))
        door_depth = depthmap.from_planemap(plane_image.crop((x, y, x + w, y + h)))
        b.create(basename + '/door', door_img, door_depth, door_shape, 1)

    return b

def init(asset_path):
    structures = loader(asset_path, 'structures')

    wall = do_wall_parts('wood_wall',
            structures('wood_wall.png'), structures('wood_wall-planemap.png'),
            mk_door=True)

    i = item_builder()
    i.merge(mk_structure_item(wall['wood_wall/edge/horiz'],
        'wood_wall/side', 'Wooden Side', (0, 0)))
    i.merge(mk_structure_item(wall['wood_wall/corner/nw'],
        'wood_wall/corner', 'Wooden Corner', (0, 0)))
    i.merge(mk_structure_item(wall['wood_wall/tee/e'],
        'wood_wall/tee', 'Wooden Tee', (0, 0)))
    i.merge(mk_structure_item(wall['wood_wall/cross'],
        'wood_wall/cross', 'Wooden Cross', (0, 0)))
    i.recipe('anvil', {'wood': 5})

    mk_structure_item(wall['wood_wall/door'], 'wood_door', 'Wooden Door') \
            .recipe('anvil', {'wood': 15})
