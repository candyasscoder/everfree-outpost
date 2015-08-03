from ..core.builder import *
from ..core import depthmap
from ..core.images import loader
from ..core.structure import Shape
from ..core.util import chop_image, chop_image_named, chop_terrain, stack, extract

from .lib.terrain import *


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

def pack4(x, base):
    a, b, c, d = x
    return a + base * (b + base * (c + base * (d)))

def unpack4(n, base):
    a = n % base; n //= base
    b = n % base; n //= base
    c = n % base; n //= base
    d = n % base; n //= base
    return (a, b, c, d)

def mk_cave_walls2_top_parts(img):
    tile = TILE_SIZE
    half = TILE_SIZE // 2

    outer = img.crop((0, 0, 3 * tile, 3 * tile))
    outer.paste((0, 0, 0, 0), (half, half, 5 * half, 5 * half))
    # Each port of the key is 1 or 0 indicating presence or absence in that
    # slot.  Slots are ordered NW, NE, SE, SW.
    OUTER_PARTS = (
            ((1, 1, 0, 1), (1, 1, 0, 0), (1, 1, 1, 0)),
            ((1, 0, 0, 1), None,         (0, 1, 1, 0)),
            ((1, 0, 1, 1), (0, 0, 1, 1), (0, 1, 1, 1)),
    )

    inner = Image.new('RGBA', (3 * tile, 3 * tile))
    crop = img.crop((half, half, 5 * half, 5 * half))
    inner.paste(crop, (half, half))
    INNER_PARTS = (
            ((0, 0, 1, 0), None,         (0, 0, 0, 1)),
            (None,         (0, 0, 0, 0), None),
            ((0, 1, 0, 0), None,         (1, 0, 0, 0)),
    )

    dct = {}
    dct.update((pack4(k, 2), v)
            for k,v in chop_image_named(outer, OUTER_PARTS).items()
            if k is not None)
    dct.update((pack4(k, 2), v)
            for k,v in chop_image_named(inner, INNER_PARTS).items()
            if k is not None)

    dct[1 | 4] = dct[1].copy()
    dct[1 | 4].paste(dct[4], (0, 0), dct[4])
    dct[2 | 8] = dct[2].copy()
    dct[2 | 8].paste(dct[8], (0, 0), dct[8])

    black = dct.pop(0).getpixel((0, 0))
    black_dct = {}
    for i in range(16):
        black_img = Image.new('RGBA', (tile, tile))
        for bit, (ox, oy) in zip(unpack4(i, 2), ((0, 0), (1, 0), (1, 1), (0, 1))):
            if bit == 1:
                x = ox * half
                y = oy * half
                black_img.paste(black, (x, y, x + half, y + half))
        black_dct[i] = black_img

    return (black_dct, dct, dct)

CAVE_WALLS2_MAX = 3 * 3 * 3 * 3
def mk_cave_walls2_tops(img):
    a_dct, b_dct, c_dct = mk_cave_walls2_top_parts(img)

    result = [None] * CAVE_WALLS2_MAX

    out = Image.new('RGBA', (CAVE_WALLS2_MAX * TILE_SIZE, TILE_SIZE))

    for i in range(CAVE_WALLS2_MAX):
        idxs = unpack4(i, 3)
        a = pack4(tuple(int(x == 0) for x in idxs), 2)
        b = pack4(tuple(int(x == 1) for x in idxs), 2)
        c = pack4(tuple(int(x == 2) for x in idxs), 2)

        result[i] = Image.new('RGBA', (TILE_SIZE, TILE_SIZE))
        def maybe_paste(x, x_dct):
            if x in x_dct:
                result[i].paste(x_dct[x], (0, 0), x_dct[x])
        maybe_paste(a, a_dct)
        maybe_paste(b, b_dct)
        maybe_paste(c, c_dct)
        out.paste(result[i], (i * TILE_SIZE, 0))

    out.save('test.png')
    return result

TERRAIN_KEYS = (
        'outside',
        'corner/outer/se',
        'corner/outer/sw',
        'edge/s',
        'corner/outer/nw',
        'cross/nw',
        'edge/w',
        'corner/inner/sw',
        'corner/outer/ne',
        'edge/e',
        'cross/ne',
        'corner/inner/se',
        'edge/n',
        'corner/inner/ne',
        'corner/inner/nw',
        'center',
        )

def mk_cave_walls2(cave_img, grass_img, dirt_img, dirt2_img, dirt2_cross_img, basename):
    tops = mk_cave_walls2_tops(cave_img)
    grass = chop_terrain(grass_img)
    dirt = chop_terrain(dirt_img)
    dirt2 = chop_terrain(dirt2_img)
    dirt2['cross/nw'] = extract(dirt2_cross_img, (0, 0))
    dirt2['cross/ne'] = extract(dirt2_cross_img, (0, 1))
    dirt2['outside'] = Image.new('RGBA', (TILE_SIZE, TILE_SIZE))
    dirt2['center'] = dirt2['center/v0']

    base_grass = dict((k, stack(grass['center/v0'], v)) for k,v in dirt2.items())
    base_dirt = dict((k, stack(dirt['center/v0'], v)) for k,v in dirt2.items())

    out = Image.new('RGBA', (CAVE_WALLS2_MAX * TILE_SIZE, 3 * TILE_SIZE))

    fronts = {
            'left': extract(cave_img, (0, 3), (1, 2)),
            'center': extract(cave_img, (1, 3), (1, 2)),
            'right': extract(cave_img, (2, 3), (1, 2)),
            }
    fronts['half_left'] = fronts['center'].copy()
    fronts['half_left'].paste((0, 0, 0, 0), (TILE_SIZE // 2, 0, TILE_SIZE, TILE_SIZE * 2))
    fronts['half_right'] = fronts['center'].copy()
    fronts['half_right'].paste((0, 0, 0, 0), (0, 0, TILE_SIZE // 2, TILE_SIZE * 2))

    front_parts = (
            dict((k, v.crop((0, TILE_SIZE, TILE_SIZE, 2 * TILE_SIZE))) for k,v in fronts.items()),
            dict((k, v.crop((0, 0, TILE_SIZE, TILE_SIZE))) for k,v in fronts.items()),
            )

    empty = Image.new('RGBA', (TILE_SIZE, TILE_SIZE))

    blks = block_builder()

    for i in range(CAVE_WALLS2_MAX):
        idxs = unpack4(i, 3)
        # Reverse so that the 0b____ constants have the bits in the usual order
        # (NW on the left, SW on the right)
        b = pack4(tuple(int(x == 1) for x in reversed(idxs)), 2)
        c = pack4(tuple(int(x == 2) for x in reversed(idxs)), 2)
        base_key = TERRAIN_KEYS[pack4(tuple(int(x != 1) for x in idxs), 2)]

        check = lambda x: b == x or c == x

        front_key = None
        if check(0b1011):
            front_key = 'left'
        elif check(0b0111):
            front_key = 'right'
        elif check(0b0011):
            front_key = 'center'
        else:
            hl = check(0b0001) or check(0b0101)
            hr = check(0b0010) or check(0b1010)
            if hl and hr:
                front_key = 'center'
            elif hl:
                front_key = 'half_left'
            elif hr:
                front_key = 'half_right'

        shape0 = 'solid' if b != 0b1111 and c != 0b1111 else 'floor'
        shape1 = 'solid' if b != 0b1111 and c != 0b1111 else 'empty'

        blks.create('%s/%d/z1' % (basename, i), shape1,
                dict(top=tops[i], front=front_parts[1].get(front_key, empty)))
        blks.create('%s/%d/z0/dirt' % (basename, i), shape0,
                dict(front=front_parts[0].get(front_key, empty), bottom=base_dirt[base_key]))
        blks.create('%s/%d/z0/grass' % (basename, i), shape0,
                dict(front=front_parts[0].get(front_key, empty), bottom=base_grass[base_key]))


    entrance_flat = extract(cave_img, (0, 5), (3, 2))
    entrance = entrance_flat.crop((TILE_SIZE // 2, 0, TILE_SIZE * 5 // 2, TILE_SIZE * 2))
    entrance_corner = extract(cave_img, (0, 3), (3, 2))
    entrance_corner.paste(entrance, (TILE_SIZE // 2, 0))

    def entrance_part(idxs, side, base_key, img):
        i = pack4(idxs, 3)
        base_key = TERRAIN_KEYS[pack4(tuple(int(x != 1) for x in idxs), 2)]
        shape0 = 'solid' if side != 'center' else 'floor'
        shape1 = 'solid' if side != 'center' else 'empty'
        x = dict(left=0, center=1, right=2)[side]
        blks.create('%s/entrance/%s/%d/z1' % (basename, side, i), shape1,
                dict(top=tops[i], front=extract(img, (x, 0))))
        blks.create('%s/entrance/%s/%d/z0/dirt' % (basename, side, i), shape0,
                dict(front=extract(img, (x, 1)), bottom=base_dirt[base_key]))
        blks.create('%s/entrance/%s/%d/z0/grass' % (basename, side, i), shape0,
                dict(front=extract(img, (x, 1)), bottom=base_grass[base_key]))

    entrance_part((0, 2, 1, 1), 'left', 'center/v0', entrance_flat)
    entrance_part((1, 2, 1, 1), 'left', 'center/v0', entrance_corner)
    entrance_part((2, 2, 1, 1), 'left', 'center/v0', entrance_flat)

    entrance_part((2, 2, 1, 1), 'center', 'center/v0', entrance_flat)

    entrance_part((2, 0, 1, 1), 'right', 'center/v0', entrance_flat)
    entrance_part((2, 1, 1, 1), 'right', 'center/v0', entrance_corner)
    entrance_part((2, 2, 1, 1), 'right', 'center/v0', entrance_flat)

    return blks

def mk_cave_top2(top_img, top_cross_img, basename):
    top = chop_terrain(top_img)
    top['cross/nw'] = extract(top_cross_img, (0, 0))
    top['cross/ne'] = extract(top_cross_img, (0, 1))
    top['outside'] = Image.new('RGBA', (TILE_SIZE, TILE_SIZE))
    top['center'] = top['center/v0']

    blks = block_builder()

    for i, k in enumerate(TERRAIN_KEYS):
        blks.create('%s/%d' % (basename, i), 'floor', dict(bottom=top[k]))

def init():
    tiles = loader('tiles')

    cave2 = tiles('lpc-cave-walls2.png')
    grass = tiles('lpc-base-tiles/grass.png')
    dirt = tiles('lpc-base-tiles/dirt.png')
    dirt2 = tiles('lpc-base-tiles/dirt2.png')
    dirt2_cross = tiles('lpc-dirt2-cross.png')

    mk_cave_walls2(cave2, grass, dirt, dirt2, dirt2_cross, 'cave')

    top = tiles('lpc-cave-top.png')
    top_cross = tiles('lpc-cave-top-cross.png')
    mk_cave_top2(top, top_cross, 'cave_top')
