import time 

OFFSETS = [
        ( 1,  0),
        ( 1,  1),
        ( 0,  1),
        (-1,  1),
        (-1,  0),
        (-1, -1),
        ( 0, -1),
        ( 1, -1),
        ]

VERTEX_OFFSETS = [
        ( 0,  0),
        ( 1,  0),
        ( 1,  1),
        ( 0,  1),
        ]

BLOCK_CORNER = {
        0: 'nw',
        1: 'ne',
        2: 'se',
        3: 'sw',
        }

def get_center_dir(bits):
    dirs = set(i for i in range(-1, 9) if (bits & (1 << (i & 7))) != 0)

    # Figure out: of the four 2x2 blocks that meet at the center point, which
    # are completely filled in?
    block_bits = 0
    for i in (1, 3, 5, 7):
        if i in dirs and i - 1 in dirs and i + 1 in dirs:
            block_bits |= 1 << (i // 2)
    return get_vertex_dir(block_bits)

def get_vertex_dir(bits):
    blocks = tuple(i for i in range(4) if (bits & (1 << i)) != 0)

    if len(blocks) == 0:
        return 'outside'
    elif len(blocks) == 1:
        return 'corner/outer/' + BLOCK_CORNER[blocks[0]]
    elif len(blocks) == 2:
        dct = {
                (0, 2): 'cross/nw',
                (1, 3): 'cross/ne',
                (0, 1): 'edge/n',
                (1, 2): 'edge/e',
                (2, 3): 'edge/s',
                (0, 3): 'edge/w',
                }
        return dct[blocks]
    elif len(blocks) == 3:
        missing = {0, 1, 2, 3}.difference(blocks).pop()
        return 'corner/inner/' + BLOCK_CORNER[(missing + 2) % 4]
    elif len(blocks) == 4:
        return 'center'
    else:
        assert False, 'unreachable'

TILE_NAMES = (
    ['outside', 'center'] +
    ['edge/%s' % d for d in 'nsew'] +
    ['corner/%s/%s' % (m, d) for m in ('inner', 'outer')
        for d in ('nw', 'ne', 'sw', 'se')] +
    ['cross/nw', 'cross/ne']
    )

TILE_ID_MAP = dict((n, i) for i, n in enumerate(TILE_NAMES))

def main():
    now = time.strftime('%Y-%m-%d %H:%M:%S')
    gen_str = 'Generated %s by util/gen_border_shape_table.py' % now

    print('// %s' % gen_str)
    print('const BORDER_SHAPE_TABLE: [u8; 256] = [')
    for base in range(0, 256, 16):
        line = ', '.join('%2d' % TILE_ID_MAP[get_center_dir(b)] for b in range(base, base + 16))
        print('    %s,' % line)
    print('];')

    print('')

    print('// %s' % gen_str)
    print('const VERTEX_SHAPE_TABLE: [u8; 16] = [')
    print('    %s,' % (', '.join('%2d' % TILE_ID_MAP[get_vertex_dir(b)] for b in range(16))))
    print('];')

    print('')

    print('// %s' % gen_str)
    print('const BORDER_TILE_NAMES: [&\'static str; 16] = [')
    for i in range(16):
        print('    "%s",' % get_vertex_dir(i))
    print('];')

    print('')

    print('// %s' % gen_str)
    print('const OFFSET_TABLE: [V2; 8] = [')
    for (x, y) in OFFSETS:
        print('    V2 { x: %2d, y: %2d },' % (x, y))
    print('];')

    print('')

    print('// %s' % gen_str)
    print('const VERTEX_OFFSET_TABLE: [V2; 4] = [')
    for (x, y) in VERTEX_OFFSETS:
        print('    V2 { x: %2d, y: %2d },' % (x, y))
    print('];')

    print('')

    print('-- %s' % gen_str)
    print('local TILE_ID_MAP = {')
    for n in TILE_NAMES:
        print("    '%s'," % n)
    print('}')


if __name__ == '__main__':
    main()
