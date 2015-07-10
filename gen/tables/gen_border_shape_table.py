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

BLOCK_CORNER = {
        1: 'nw',
        3: 'ne',
        5: 'se',
        7: 'sw',
        }

def get_dir(bits):
    dirs = set(i for i in range(-1, 9) if (bits & (1 << (i & 7))) != 0)

    # Figure out: of the four 2x2 blocks that meet at the center point, which
    # are completely filled in?
    blocks = set()
    for i in (1, 3, 5, 7):
        if i in dirs and i - 1 in dirs and i + 1 in dirs:
            blocks.add(i)
    blocks = tuple(sorted(blocks))

    if len(blocks) == 0:
        return 'outside'
    elif len(blocks) == 1:
        return 'corner/outer/' + BLOCK_CORNER[blocks[0]]
    elif len(blocks) == 2:
        dct = {
                (1, 5): 'cross/nw',
                (3, 7): 'cross/ne',
                (1, 3): 'edge/n',
                (3, 5): 'edge/e',
                (5, 7): 'edge/s',
                (1, 7): 'edge/w',
                }
        return dct[blocks]
    elif len(blocks) == 3:
        missing = {1, 3, 5, 7}.difference(blocks).pop()
        return 'corner/inner/' + BLOCK_CORNER[(missing + 4) % 8]
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
        line = ', '.join('%2d' % TILE_ID_MAP[get_dir(b)] for b in range(base, base + 16))
        print('    %s,' % line)
    print('];')

    print('')

    print('// %s' % gen_str)
    print('const OFFSET_TABLE: [V2; 8] = [')
    for (x, y) in OFFSETS:
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
