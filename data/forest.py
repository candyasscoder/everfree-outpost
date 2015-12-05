from outpost_data.core import structure
from outpost_data.core.consts import *
from outpost_data.core.builder2 import STRUCTURE
from outpost_data.core.image2 import loader
from outpost_data.outpost.lib import terrain2, models

TREE_SHAPE = structure.Shape(3, 3, 4, [
        'empty', 'empty', 'empty',
        'empty', 'solid', 'empty',
        'empty', 'empty', 'empty',

        'empty', 'empty', 'empty',
        'empty', 'solid', 'empty',
        'empty', 'empty', 'empty',

        'solid', 'solid', 'solid',
        'solid', 'solid', 'solid',
        'solid', 'solid', 'solid',

        'solid', 'solid', 'solid',
        'solid', 'solid', 'solid',
        'solid', 'solid', 'solid',
        ])

STUMP_SHAPE = structure.Shape(3, 3, 1, [
        'empty', 'empty', 'empty',
        'empty', 'solid', 'empty',
        'empty', 'empty', 'empty',
        ])

def init():
    tiles = loader('tiles', unit=TILE_SIZE)

    terrain2.terrain_blocks('grass', tiles('lpc-base-tiles/grass.png'), shape='floor')
    terrain2.terrain_blocks('water_grass', tiles('lpc-base-tiles/watergrass.png'))
    terrain2.terrain_cross_blocks('water_grass', tiles('lpc-watergrass-cross.png'))

    terrain2.interior_blocks('farmland', tiles('farmland-interior-parts.png'), shape='floor')

    structures = loader('structures', unit=TILE_SIZE)
    s = STRUCTURE.prefixed('tree') \
            .shape(TREE_SHAPE) \
            .layer(1)

    s.new('v0') \
            .part(models.TREE['shadow'], structures('tree-shadow-round.png')) \
            .part(models.TREE['trunk'], structures('tree-trunk.png')) \
            .part(models.TREE['top'], structures('tree-top-round.png'))

    s.new('v1') \
            .part(models.TREE['shadow'], structures('tree-shadow-cone.png')) \
            .part(models.TREE['trunk'], structures('tree-trunk.png')) \
            .part(models.TREE['top'], structures('tree-top-cone.png'))

    STRUCTURE.new('stump') \
            .shape(STUMP_SHAPE) \
            .layer(1) \
            .part(models.TREE['stump'], structures('tree-stump.png'))
