from outpost_data.builder import *
import outpost_data.images as I
from outpost_data import depthmap
from outpost_data.structure import Shape
from outpost_data.util import loader, chop_image, chop_terrain, stack

from lib.terrain import *


def mk_cave_walls(img_grass, img_dirt, img_cave_walls, basename):
    grass = chop_terrain(img_grass)['center/v0']
    dirt = chop_terrain(img_dirt)
    walls = chop_image(img_cave_walls)
    name = lambda n: '%s/%s' % (basename, n)
    w = lambda x, y: walls[(x, y)]

    blks = block_builder()

    def wall(n, t, f1, f0, b='default'):
        if b == 'default':
            b = stack(grass, dirt[n])
        blks.create(name(n) + '/z1', 'solid', {'top': t, 'front': f1})
        blks.create(name(n) + '/z0', 'solid', {'front': f0, 'bottom': b})

    blks.create(name('center/z1'), 'empty', {})
    blks.create(name('center/z0'), 'floor', {'bottom': dirt['center/v0']})

    wall('edge/n', w(2, 1), w(2, 2), w(2, 3))
    wall('edge/s', w(2, 1), w(2, 2), w(2, 3))
    wall('edge/w', w(2, 0), None, None)
    wall('edge/e', w(2, 0), None, None)

    wall('corner/outer/nw', w(0, 0), w(3, 0), w(3, 1))
    wall('corner/outer/ne', w(1, 0), w(4, 0), w(4, 1))
    wall('corner/outer/sw', w(0, 1), w(0, 2), w(0, 3))
    wall('corner/outer/se', w(1, 1), w(1, 2), w(1, 3))

    wall('corner/inner/se', w(0, 0), w(3, 0), w(3, 1))
    wall('corner/inner/sw', w(1, 0), w(4, 0), w(4, 1))
    wall('corner/inner/ne', w(0, 1), w(0, 2), w(0, 3))
    wall('corner/inner/nw', w(1, 1), w(1, 2), w(1, 3))

    # TODO: need dirt cross tiles for the base
    wall('cross/nw', w(5, 1), w(2, 2), w(2, 3), dirt['center/v0'])
    wall('cross/ne', w(5, 1), w(2, 2), w(2, 3), dirt['center/v0'])

    return blks

def mk_cave_entrance(img_grass, img_dirt, img_cave_walls, basename):
    grass = chop_terrain(img_grass)['center/v0']
    dirt = chop_terrain(img_dirt)
    walls = chop_image(img_cave_walls)
    name = lambda x, z: '%s/x%d/z%d' % (basename, x, z)
    w = lambda x, y: walls[(x, y)]

    # TODO: should allow for entrances on the north side as well
    bottom = stack(grass, dirt['edge/s'])
    top = w(2, 1)

    blks = block_builder()

    parts = []
    for x in range(3):
        blks.create(name(x, 1), 'solid', {'top': top, 'front': w(3 + x, 2)})
        blks.create(name(x, 0), 'solid', {'front': w(3 + x, 3), 'bottom': bottom})

    blks[name(1, 1)].shape = 'empty'
    blks[name(1, 0)].shape = 'floor'

    return blks


def init(asset_path):
    tiles = loader(asset_path, 'tiles')

    grass = tiles('lpc-base-tiles/grass.png')
    dirt = tiles('lpc-base-tiles/dirt2.png')
    cave = tiles('lpc-cave-walls.png')
    mk_cave_walls(grass, dirt, cave, 'cave')
    mk_cave_entrance(grass, dirt, cave, 'cave_entrance')

    mk_floor_blocks(tiles('lpc-cave-top.png'), 'cave_top', shape='floor')
    mk_floor_cross(tiles('lpc-cave-top-cross.png'), 'cave_top', shape='floor')
