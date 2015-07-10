from ..core.builder import *
from ..core.images import loader
from ..core import depthmap
from ..core.structure import Shape
from ..core.util import extract

from .lib.structures import *
from .lib.terrain import *


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

    b = structure_builder()

    b.create(
        'tree',
        image.crop(tree_bounds),
        depthmap.from_planemap(plane_image.crop(tree_bounds)),
        tree_shape,
        1)

    b.create(
        'stump',
        image.crop(stump_bounds),
        depthmap.from_planemap(plane_image.crop(stump_bounds)),
        stump_shape,
        1)

    return b


def init():
    tiles = loader('tiles')
    structures = loader('structures')
    daneeklu = loader('tiles/daneeklu_farming_tilesets')
    lpc = loader('tiles/lpc-base-tiles')

    mk_floor_blocks(tiles('lpc-base-tiles/grass.png'), 'grass', shape='floor')
    mk_floor_blocks(tiles('lpc-base-tiles/watergrass.png'), 'water_grass')
    mk_floor_cross(tiles('lpc-watergrass-cross.png'), 'water_grass')

    do_tree(structures('tree.png'), structures('tree-planemap.png'))
    mk_solid_structure('rock', structures('rock.png'), (2, 1, 1))


    mk_item('wood', 'Wood', extract(daneeklu('farming_fishing.png'), (5, 1)))
    mk_item('stone', 'Stone', extract(lpc('rock.png'), (0, 0)))


    mk_terrain_interior(tiles('farmland-interior-parts.png'), 'farmland', shape='floor')
