import time 

OFFSETS = (
        ( 1,  0),
        ( 1,  1),
        ( 0,  1),
        (-1,  1),
        (-1,  0),
        (-1, -1),
        ( 0, -1),
        ( 1, -1),
        )

ORTHO_OFFSETS = tuple(OFFSETS[0::2])
CORNER_OFFSETS = tuple(OFFSETS[1::2])

CORNER_NAMES = {
        1: 'nw',
        3: 'ne',
        5: 'se',
        7: 'sw',
        }

DIR_NAMES = ('n', 'w', 's', 'e')

CORNERS_BY_NAME = {'nw': 0, 'sw': 1, 'se': 2, 'ne': 3}

def explode(x, n):
    return tuple(i for i in range(n) if ((x >> i) & 1) == 1)

def name_from_bits(bits):
    return (
            ('n' if 0 in bits else '') +
            ('s' if 2 in bits else '') +
            ('w' if 1 in bits else '') +
            ('e' if 3 in bits else '')
            )

def full_table_entry(i):
    edges = explode(i, 4)
    corners = explode(i >> 4, 4)

    base = name_from_bits(edges)
    full_corners = ()
    if len(edges) == 0:
        base = 'spot'
    elif len(edges) == 2:
        if base not in ('ns', 'we'):
            full_corners = (CORNERS_BY_NAME[base],)
    elif len(edges) == 3:
        if base.startswith('ns'):
            full_corners = (
                    CORNERS_BY_NAME[base[0] + base[2]],
                    CORNERS_BY_NAME[base[1] + base[2]],
                    )
        else:
            full_corners = (
                    CORNERS_BY_NAME[base[0] + base[1]],
                    CORNERS_BY_NAME[base[0] + base[2]],
                    )
    elif len(edges) == 4:
        full_corners = (0, 1, 2, 3)

    full_corners = tuple(sorted(full_corners))

    if len(full_corners) > 0:
        return base + '/' + ''.join('1' if c in corners else '0' for c in full_corners)
    else:
        return base

def key(s):
    base, _, corners = s.partition('/')
    return (len(base) if base != 'spot' else 0, -len(corners), base, corners)

def main():
    now = time.strftime('%Y-%m-%d %H:%M:%S')
    gen_str = 'Generated %s by util/gen_border_shape_table.py' % now

    table = list(full_table_entry(i) for i in range(256))
    name_order = sorted(set(table), key=key)
    index_map = dict((n, i) for i, n in enumerate(name_order))

    print('// %s' % gen_str)
    print('const INTERIOR_SHAPE_TABLE: [u8; 256] = [')
    for base in range(0, 256, 16):
        line = ', '.join('%2d' % index_map[table[i]] for i in range(base, base + 16))
        print('    %s,' % line)
    print('];')

    print('')

    print('// %s' % gen_str)
    print('const INTERIOR_NAMES: [&\'static str; %d] = [' % len(name_order))
    for n in name_order:
        print('    "%s",' % n)
    print('];')

    print('')

    print('-- %s' % gen_str)
    print('local INTERIOR_NAMES = {')
    for n in name_order:
        print("    '%s'," % n)
    print('}')



if __name__ == '__main__':
    main()
