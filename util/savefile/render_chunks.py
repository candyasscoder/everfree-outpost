import argparse
import json
import os
import sys

from PIL import Image

from parse import points


def log(s):
    sys.stderr.write('%s\n' % s)


def build_parser():
    parser = argparse.ArgumentParser(
            description='Render the contents of a group of chunks')

    parser.add_argument('--block-data', metavar='FILE',
            help='path to the client tiles.json')

    parser.add_argument('--atlas', metavar='FILE',
            help='path to the tile atlas image')

    parser.add_argument('--out', metavar='FILE',
            help='path to save the rendered image')

    parser.add_argument('--tile', metavar='SIZE',
            help='divide output into SIZExSIZE-chunk regions')

    return parser


def render(data, area, out_file):
    chunk_map, blocks, atlas = data
    min_x, max_x, min_y, max_y = area

    w = 32 * 16 * (max_x - min_x)
    h = 32 * 16 * (max_y - min_y + 1)
    img = Image.new('RGBA', (w, h))

    def draw_tile(idx, dx, dy):
        if idx == 0:
            return

        sx = 32 * (idx % 32)
        sy = 32 * (idx // 32)
        src = atlas.crop((sx, sy, sx + 32, sy + 32))
        img.paste(src, (dx, dy), src)

    saw_chunk = False
    for cy in range(min_y, max_y):
        for cx in range(min_x, max_x):
            if (cx, cy) not in chunk_map:
                continue
            saw_chunk = True

            chunk = chunk_map[(cx, cy)]
            base_x = (cx - min_x) * 32 * 16
            base_y = (cy - min_y + 1) * 32 * 16

            for i, (dx, dy, dz) in enumerate(points(16, 16, 16)):
                b = chunk[i]
                if b == 0:
                    continue

                du = dy + dz
                dv = dy - dz

                dest_x = base_x + dx * 32
                dest_y = base_y + dv * 32

                for side in ('bottom', 'front'):
                    draw_tile(blocks[b].get(side, 0), dest_x, dest_y)
                for side in ('back', 'top'):
                    draw_tile(blocks[b].get(side, 0), dest_x, dest_y - 32)

    if saw_chunk:
        img.save(out_file)

def render_tiled(data, area, out_file, tile_size):
    min_x, max_x, min_y, max_y = area

    min_tx = min_x // tile_size
    max_tx = (max_x + tile_size - 1) // tile_size

    min_ty = min_y // tile_size
    max_ty = (max_y + tile_size - 1) // tile_size

    count = (max_tx - min_tx) * (max_ty - min_ty)
    idx = 0

    for ty in range(min_ty, max_ty):
        for tx in range(min_tx, max_tx):
            idx += 1
            log('%4d / %4d' % (idx, count))
            base_x = tx * tile_size
            base_y = ty * tile_size
            tile_area = (base_x, base_x + tile_size, base_y, base_y + tile_size)

            base, _, ext = out_file.partition('.')
            tile_out_file = '%s-%d,%d.%s' % (base, tx, ty, ext)
            render(data, tile_area, tile_out_file)


def main():
    parser = build_parser()
    args = parser.parse_args()

    with open(os.path.join(args.block_data)) as f:
        blocks = json.load(f)['blocks']
    atlas = Image.open(args.atlas)

    j = json.load(sys.stdin)
    chunk_map = dict((tuple(map(int, k.split(','))), v) for k,v in j.items())

    min_x = min(x for x,y in chunk_map.keys())
    max_x = 1 + max(x for x,y in chunk_map.keys())
    min_y = min(y for x,y in chunk_map.keys())
    max_y = 1 + max(y for x,y in chunk_map.keys())

    data = (chunk_map, blocks, atlas)
    area = (min_x, max_x, min_y, max_y)
    if args.tile is None:
        render(data, area, args.out)
    else:
        render_tiled(data, area, args.out, int(args.tile))


if __name__ == '__main__':
    main()
