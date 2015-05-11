import time 

def gen_blocked_sections():
    result = []

    for i in range(1 << 6):
        x_edge = (i & 0x01) != 0
        y_edge = (i & 0x02) != 0
        z_edge = (i & 0x04) != 0
        x_mid = (i & 0x08) != 0
        y_mid = (i & 0x10) != 0
        z_mid = (i & 0x20) != 0

        out_mid = x_mid and y_mid and z_mid
        out_x = x_edge and y_mid and z_mid
        out_y = y_edge and x_mid and z_mid
        out_z = z_edge and x_mid and y_mid
        out_xy = x_edge and y_edge and z_mid
        out_xz = x_edge and z_edge and y_mid
        out_yz = y_edge and z_edge and x_mid
        out_xyz = x_edge and y_edge and z_edge

        out = int(out_mid) << 0 | \
              int(out_x) << 1 | \
              int(out_y) << 2 | \
              int(out_xy) << 3 | \
              int(out_z) << 4 | \
              int(out_xz) << 5 | \
              int(out_yz) << 6 | \
              int(out_xyz) << 7

        result.append(out)

    return result

def gen_blocking():
    result = []

    for i in range(1 << 8):
        mid =   (i & (1 << 0)) != 0
        x =     (i & (1 << 1)) != 0
        y =     (i & (1 << 2)) != 0
        xy =    (i & (1 << 3)) != 0
        z =     (i & (1 << 4)) != 0
        xz =    (i & (1 << 5)) != 0
        yz =    (i & (1 << 6)) != 0
        xyz =   (i & (1 << 7)) != 0

        out = 0

        if mid:
            out |= 1 | 2 | 4
        elif x or y or z:
            out |= 1 if x else 0
            out |= 2 if y else 0
            out |= 4 if z else 0
        elif xy or xz or yz:
            out |= 1 | 2 if xy else 0
            out |= 1 | 4 if xz else 0
            out |= 2 | 4 if yz else 0
        elif xyz:
            out |= 1 | 2 | 4

        result.append(out)

    return result

def main():
    now = time.strftime('%Y-%m-%d %H:%M:%S')
    gen_str = 'Generated %s by util/gen_physics_tables.py' % now

    table = gen_blocked_sections()
    print('// %s' % gen_str)
    print('const BLOCKED_SECTIONS_TABLE: [u8; 64] = [')
    for base in range(0, len(table), 16):
        line = ', '.join('0x%02x' % table[i] for i in range(base, base + 16))
        print('    %s,' % line)
    print('];\n')

    table = gen_blocking()
    print('// %s' % gen_str)
    print('const BLOCKING_TABLE: [u8; 256] = [')
    for base in range(0, len(table), 16):
        line = ', '.join('0x%02x' % table[i] for i in range(base, base + 16))
        print('    %s,' % line)
    print('];\n')


if __name__ == '__main__':
    main()
