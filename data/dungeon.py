from outpost_data.builder import *
import outpost_data.images as I
from outpost_data import depthmap
from outpost_data.structure import Shape
from outpost_data.util import loader, chop_image_named, chop_terrain, stack

from lib.terrain import *


def mk_cave_inside(img, basename, dirt):
    name = lambda n: '%s/%s' % (basename, n)
    blks = block_builder()

    extra_parts = (
            ('front/left/z1', 'front/center/z1', 'front/right/z1'),
            ('front/left/z0', 'front/center/z0', 'front/right/z0'),
            )
    p = chop_image_named(img, TERRAIN_PARTS + extra_parts)
    p['black'] = p['center/v3']

    def wall(n, side=None):
        if side is not None:
            f1 = p['front/%s/z1' % side]
            f0 = p['front/%s/z0' % side]
        else:
            f1 = None
            f0 = None
        blks.create(name(n) + '/z1', 'solid', {'top': p[n], 'front': f1})
        blks.create(name(n) + '/z0', 'solid', {'front': f0, 'bottom': dirt})

    blks.create(name('center/z1'), 'empty', {})
    blks.create(name('center/z0'), 'floor', {'bottom': dirt})

    blks.create(name('outside/z1'), 'solid', {'top': p['black']})
    blks.create(name('outside/z0'), 'solid', {})

    wall('edge/n', 'center')
    wall('edge/s')
    wall('edge/e')
    wall('edge/w')

    wall('corner/outer/nw', 'center')
    wall('corner/outer/ne', 'center')
    wall('corner/outer/sw')
    wall('corner/outer/se')

    wall('corner/inner/nw')
    wall('corner/inner/ne')
    wall('corner/inner/sw', 'left')
    wall('corner/inner/se', 'right')

    return blks

def init(asset_path):
    tiles = loader(asset_path, 'tiles')

    dirt2 = chop_terrain(tiles('lpc-base-tiles/dirt2.png'))
    cave_floor = dirt2['center/v0']

    mk_cave_inside(tiles('lpc-cave-inside.png'), 'cave_inside', cave_floor)

    mk_floor_blocks(tiles('lpc-base-tiles/water.png'), 'cave_water', base_img=cave_floor)

    mk_floor_blocks(tiles('lpc-base-tiles/lava.png'), 'cave_lava', base_img=cave_floor) \
            .light((255, 100, 0), 50)

    mk_floor_blocks(tiles('lpc-base-tiles/holemid.png'), 'cave_pit', base_img=cave_floor)
