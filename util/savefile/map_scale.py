import os
import sys

import shutil
from PIL import Image


def log(s):
    sys.stderr.write('%s\n' % s)

def log2(x):
    i = 0
    while x > 1:
        x = (x + 1) >> 1
        i += 1
    return i

# The size of a single map tile
SIZE = 256

def zoom_out(z, max_x, max_y, outer_tiles):
    cur = 'z%d' % z
    outer = 'z%d' % (z + 1)
    tiles = set((x // 2, y // 2) for x,y in outer_tiles)

    for i, (x,y) in enumerate(tiles):
        log('  [z %2d] tile %5d / %5d' % (z, i, len(tiles)))

        img = Image.new('RGBA', (2 * SIZE, 2 * SIZE))
        for dx in (0, 1):
            for dy in (0, 1):
                outer_x = x * 2 + dx
                outer_y = y * 2 + dy
                if (outer_x, outer_y) not in outer_tiles:
                    continue

                outer_path = os.path.join(outer, '%d,%d.png' % (outer_x, outer_y))
                src = Image.open(outer_path)
                img.paste(src, (dx * SIZE, dy * SIZE))

        img = img.resize((SIZE, SIZE), Image.BILINEAR)
        img.save(os.path.join(cur, '%d,%d.png' % (x, y)))
        tiles.add((x, y))
    return tiles

def main():
    files = os.listdir('orig')
    os.makedirs('max_zoom', exist_ok=True)
    tiles = []
    for f in files:
        base, _, _ = f.rpartition('.')
        _, _, coord = base.rpartition('-')
        x_str, _, y_str = coord.partition(',')
        x = int(x_str)
        y = int(y_str)
        tiles.append((x, y))
        shutil.copy(os.path.join('orig', f),
                os.path.join('max_zoom', '%d,%d.png' % (x, y)))

    max_x = max(x for x,y in tiles)
    max_y = max(y for x,y in tiles)

    max_dim = max(max_x, max_y)
    max_zoom = log2(max_dim)

    shutil.move('max_zoom', 'z%d' % max_zoom)
    max_x = (max_x + 1) >> 1
    max_y = (max_y + 1) >> 1
    outer_tiles = set(tiles)

    for z in reversed(range(max_zoom)):
        log('produce zoom level %d (%d x %d)' % (z, max_x, max_y))
        os.makedirs('z%d' % z, exist_ok=True)
        outer_tiles = zoom_out(z, max_x, max_y, outer_tiles)
        max_x = (max_x + 1) >> 1
        max_y = (max_y + 1) >> 1

if __name__ == '__main__':
    main()

