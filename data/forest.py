from outpost_data.core.consts import *
from outpost_data.core.builder2 import *
from outpost_data.core.image2 import loader
from outpost_data.outpost.lib import terrain2

def init():
    tiles = loader('tiles', unit=TILE_SIZE)

    terrain2.terrain_blocks('grass', tiles('lpc-base-tiles/grass.png'), shape='floor')
    terrain2.terrain_blocks('water_grass', tiles('lpc-base-tiles/watergrass.png'))
    terrain2.terrain_cross_blocks('water_grass', tiles('lpc-watergrass-cross.png'))

    terrain2.interior_blocks('farmland', tiles('farmland-interior-parts.png'), shape='floor')
