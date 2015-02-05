from math import ceil
import os

from PIL import Image

def compute_atlas(block_arr, tiles, keys):
    order = [()]
    rev = {}

    for block in block_arr:
        for k in keys:
            if k not in block:
                continue

            tile_names = block[k];
            layers = tuple(tiles[tile_name] for tile_name in tile_names)
            display = tuple((t['sheet'], t['x'], t['y']) for t in layers)

            if tile_names in rev:
                continue
            order.append(display)
            rev[tile_names] = len(order) - 1

    return (order, rev)

TILE_SIZE = 32
ATLAS_WIDTH = 32

def collect_sheets(order):
    return set(s for layers in order for s,_,_ in layers)

def build_atlas_image(order, in_dir):
    sheets = dict((s, None) for s in collect_sheets(order))
    for s in sheets:
        if s is None:
            continue
        sheets[s] = Image.open(os.path.join(in_dir, s))

    atlas_height = ceil(len(order) / ATLAS_WIDTH)
    px_width = ATLAS_WIDTH * TILE_SIZE
    px_height = atlas_height * TILE_SIZE
    output = Image.new('RGBA', (px_width, px_height))

    for idx, layers in enumerate(order):
        out_tx = idx % ATLAS_WIDTH
        out_ty = idx // ATLAS_WIDTH

        for (s, tx, ty) in layers:
            if sheets[s] is None:
                continue

            in_x = tx * TILE_SIZE
            in_y = ty * TILE_SIZE
            display = sheets[s].crop((in_x, in_y, in_x + TILE_SIZE, in_y + TILE_SIZE))
            output.paste(display, (out_tx * TILE_SIZE, out_ty * TILE_SIZE), mask=display)

    return output

def build_client_json(image):
    _, px_height = image.size
    atlas_height = px_height // TILE_SIZE
    opaque = [None] * (ATLAS_WIDTH * atlas_height)

    for ty in range(atlas_height):
        for tx in range(ATLAS_WIDTH):
            tile = image.crop((
                    tx * TILE_SIZE,
                    ty * TILE_SIZE,
                    (tx + 1) * TILE_SIZE,
                    (ty + 1) * TILE_SIZE))
            _, _, _, (min_a, _) = tile.getextrema()
            opaque[ty * ATLAS_WIDTH + tx] = (min_a == 255)

    return opaque

def save_image(image, path):
    image.save(path)
