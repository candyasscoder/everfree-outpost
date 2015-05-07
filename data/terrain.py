import os

from PIL import Image

from outpost_data.builder import *
from outpost_data.consts import *
import outpost_data.images as I
from outpost_data.util import *


def mk_floor_from_dict(basename, dct, shape='empty', base_img=None):
    blocks = {}
    for part_name, part_img in dct.items():
        if base_img is not None:
            img = base_img.copy()
            img.paste(part_img, (0, 0), part_img)
        else:
            img = part_img

        b = mk_block('%s/%s' % (basename, part_name), shape, {'bottom': img})
        blocks[part_name] = b
    return blocks

def mk_floor_blocks(img, basename, **kwargs):
    dct = chop_terrain(img)
    dct['center'] = dct['center/v0']
    return mk_floor_from_dict(basename, dct, **kwargs)

def mk_floor_cross(img, basename, **kwargs):
    dct = {
            'cross/nw': img.crop((0, 0, TILE_SIZE, TILE_SIZE)),
            'cross/ne': img.crop((0, TILE_SIZE, TILE_SIZE, 2 * TILE_SIZE)),
            }
    mk_floor_from_dict(basename, dct, **kwargs)


def mk_cave_walls(img_grass, img_dirt, img_cave_walls, basename):
    grass = chop_terrain(img_grass)['center/v0']
    dirt = chop_terrain(img_dirt)
    walls = chop_image(img_cave_walls)
    name = lambda n: '%s/%s' % (basename, n)
    w = lambda x, y: walls[(x, y)]

    def wall(n, t, f1, f0, b='default'):
        if b == 'default':
            b = stack(grass, dirt[n])
        z1 = mk_block(name(n) + '/z1', 'solid', {'top': t, 'front': f1})
        z0 = mk_block(name(n) + '/z0', 'solid',
                {'front': f0, 'bottom': b})
        return (z0, z1)

    mk_block(name('center/z1'), 'empty', {})
    mk_block(name('center/z0'), 'floor', {'bottom': dirt['center/v0']})

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

def mk_cave_entrance(img_grass, img_dirt, img_cave_walls, basename):
    grass = chop_terrain(img_grass)['center/v0']
    dirt = chop_terrain(img_dirt)
    walls = chop_image(img_cave_walls)
    name = lambda x, z: '%s/x%d/z%d' % (basename, x, z)
    w = lambda x, y: walls[(x, y)]

    # TODO: should allow for entrances on the north side as well
    bottom = stack(grass, dirt['edge/s'])
    top = w(2, 1)

    parts = []
    for x in range(3):
        z1 = mk_block(name(x, 1), 'solid', {'top': top, 'front': w(3 + x, 2)})
        z0 = mk_block(name(x, 0), 'solid', {'front': w(3 + x, 3), 'bottom': bottom})
        parts.append((z0, z1))

    parts[1][0].shape = 'floor'
    parts[1][1].shape = 'empty'


def mk_cave_inside(img, basename, dirt):
    name = lambda n: '%s/%s' % (basename, n)

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
        mk_block(name(n) + '/z1', 'solid', {'top': p[n], 'front': f1})
        mk_block(name(n) + '/z0', 'solid', {'front': f0, 'bottom': dirt})

    mk_block(name('center/z1'), 'empty', {})
    mk_block(name('center/z0'), 'floor', {'bottom': dirt})

    mk_block(name('outside/z1'), 'solid', {'top': p['black']})
    mk_block(name('outside/z0'), 'solid', {})

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


def init(asset_path):
    path = os.path.join(asset_path, 'tiles')
    img = lambda name: I.load(os.path.join(path, name))

    t = mk_tile('empty', Image.new('RGBA', (TILE_SIZE, TILE_SIZE)))
    mk_block('empty', 'empty', {})

    mk_floor_blocks(img('lpc-base-tiles/grass.png'), 'grass', shape='floor')
    mk_floor_blocks(img('lpc-base-tiles/watergrass.png'), 'water_grass')
    mk_floor_cross(img('lpc-watergrass-cross.png'), 'water_grass')

    grass = img('lpc-base-tiles/grass.png')
    dirt = img('lpc-base-tiles/dirt2.png')
    cave = img('lpc-cave-walls.png')
    mk_cave_walls(grass, dirt, cave, 'cave')
    mk_cave_entrance(grass, dirt, cave, 'cave_entrance')

    dirt2 = chop_terrain(img('lpc-base-tiles/dirt2.png'))
    cave_floor = dirt2['center/v0']
    mk_cave_inside(img('lpc-cave-inside.png'), 'cave_inside', cave_floor)
    mk_floor_blocks(img('lpc-base-tiles/water.png'), 'cave_water', base_img=cave_floor)

    lava = mk_floor_blocks(img('lpc-base-tiles/lava.png'), 'cave_lava', base_img=cave_floor)
    for b in lava.values():
        b.light_color = (255, 100, 0)
        b.light_radius = 50

    mk_floor_blocks(img('lpc-base-tiles/holemid.png'), 'cave_pit', base_img=cave_floor)

    mk_floor_blocks(img('lpc-cave-top.png'), 'cave_top', shape='floor')
    mk_floor_cross(img('lpc-cave-top-cross.png'), 'cave_top', shape='floor')
