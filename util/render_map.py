from collections import namedtuple
from functools import lru_cache
import io
import json
import math
import os
import struct
import sys
import time

from outpost_savegame import *

from PIL import Image


### Game data loading

class Pack(object):
    def __init__(self, f):
        self.f = f

        self.f.seek(0)
        index_len, = struct.unpack('I', self.f.read(4))
        raw_index = json.loads(self.f.read(index_len).decode('utf-8'))
        self.index = dict((x['name'], x) for x in raw_index)

        self.seek_base = 4 + index_len
        self.loaded = {}

    def get(self, name):
        if name not in self.loaded:
            entry = self.index[name]
            self.f.seek(self.seek_base + entry['offset'])
            data = self.f.read(entry['length'])

            if entry['type'] == 'json':
                data = json.loads(data.decode('utf-8'))
            elif entry['type'] == 'text':
                data = data.decode('utf-8')
            elif entry['type'] == 'image':
                data = Image.open(io.BytesIO(data))

            self.loaded[name] = data
        return self.loaded[name]

class DataDir(object):
    def __init__(self, path):
        self.path = path
        self.loaded = {}

    def get(self, name):
        if name not in self.loaded:
            with open(os.path.join(self.path, name), 'rb') as f:
                data = f.read()

            if name.endswith('.json'):
                data = json.loads(data.decode('utf-8'))

            self.loaded[name] = data
        return self.loaded[name]

BlockDef = namedtuple('BlockDef', ('top', 'bottom', 'front', 'back'))

def mk_block_data(pack, data_dir):
    blocks = {}
    server = data_dir.get('blocks.json')
    client = pack.get('block_defs')

    assert len(server) == len(client), \
            'mismatch between server and client block data'

    for i in range(len(server)):
        name = server[i]['name']
        info = client[i]
        blocks[name] = BlockDef(
                info.get('top', 0),
                info.get('bottom', 0),
                info.get('front', 0),
                info.get('back', 0))

    return blocks

TemplateDef = namedtuple('TemplateDef',
        ('layer', 'size', 'display_size', 'sheet_idx', 'offset'))

def mk_template_data(pack, data_dir):
    templates = {}
    server = data_dir.get('structures.json')
    client = pack.get('template_defs')

    assert len(server) == len(client), \
            'mismatch between server and client template data'

    for i in range(len(server)):
        name = server[i]['name']
        layer = client[i]['layer']
        size = tuple(client[i]['size'])
        disp_size = tuple(client[i]['display_size'])
        sheet_idx = client[i]['sheet']
        offset = tuple(client[i]['offset'])
        templates[name] = TemplateDef(layer, size, disp_size, sheet_idx, offset)

    return templates

Data = namedtuple('Data', ('client', 'server', 'blocks', 'templates'))

def load_data(dist_dir):
    pack = Pack(open(os.path.join(dist_dir, 'www/outpost.pack'), 'rb'))
    data_dir = DataDir(os.path.join(dist_dir, 'data'))
    blocks = mk_block_data(pack, data_dir)
    templates = mk_template_data(pack, data_dir)

    return Data(pack, data_dir, blocks, templates)


### Chunk Slicing

# x, u, v are all in tile coordinates, not pixels.
#
# NB: `u` is the `u` coordinate of the "reference point" (-y -z edge). `v` is
# adjusted to give the correct screen position, so it may not correspond to
# `x`/`y`/`z`/`u` in the normal way.
Slice = namedtuple('Slice', ('x', 'u', 'v', 'layer', 'image'))

TILE_SIZE = 32
ATLAS_SIZE = 32
CHUNK_SIZE = 16

CHUNK_PX = CHUNK_SIZE * TILE_SIZE

def extract_tile(img, idx):
    x = idx % ATLAS_SIZE
    y = idx // ATLAS_SIZE

    px = x * TILE_SIZE
    py = y * TILE_SIZE

    return img.crop((px, py, px + TILE_SIZE, py + TILE_SIZE))

def slice_block(data, x, y, z, name):
    info = data.blocks[name]
    tile_atlas = data.client.get('tiles')

    slices = []
    def add(x, y, z, adj, layer, index):
        if index != 0:
            slices.append(Slice(x, y + z, y - z + adj, layer,
                extract_tile(tile_atlas, index)))

    add(x, y, z, -1, -2, info.back)
    add(x, y, z, -1, -1, info.top)
    add(x, y, z,  0, -2, info.bottom)
    add(x, y, z,  0, -1, info.front)
    return slices

def slice_structure(data, structure):
    info = data.templates[structure.template]
    sheet = data.client.get('structures%d' % info.sheet_idx)

    ox, oy = info.offset
    dx, dy = info.display_size
    assert dx % TILE_SIZE == 0 and dy % TILE_SIZE == 0, \
            "can't handle structures that take up a non-whole number of tiles"

    # `size` is already measured in tiles
    sx, sy, sz = info.size

    pos = structure.offset

    slices = []
    def add(x, y, z, adj, img):
        slices.append(Slice(x, y + z, y - z + adj, info.layer, img))

    for i in range(dy // TILE_SIZE):
        src_y = oy + dy - (i + 1) * TILE_SIZE
        img = sheet.crop((ox, src_y, ox + dx, src_y + TILE_SIZE))
        if i < sz:
            # Front face of the structure
            add(pos.x, pos.y + sy - 1, pos.z + i, 0, img)
        else:
            # Top face of the structure
            j = i - sz
            add(pos.x, pos.y + sy - (j + 1) , pos.z + sz - 1, -1, img)

    return slices

def slice_chunk(data, chunk):
    slices = []

    for z in range(CHUNK_SIZE):
        for y in range(CHUNK_SIZE):
            for x in range(CHUNK_SIZE):
                idx = x + CHUNK_SIZE * (y + CHUNK_SIZE * z)
                block_name = chunk.blocks[idx]
                if block_name == 'empty':
                    continue
                slices.extend(slice_block(data, x, y, z, block_name))

    for s in chunk.child_structures:
        slices.extend(slice_structure(data, s))

    return slices



### Rendering

def render_slices(img, slices, offset):
    slices.sort(key=lambda s: (s.u, s.v, s.layer, s.x))
    ox, oy = offset
    for s in slices:
        x = (s.x + ox) * TILE_SIZE
        y = (s.v + oy) * TILE_SIZE
        w, h = s.image.size

        if x + w <= 0 or x >= CHUNK_PX or y + h <= 0 or y >= CHUNK_PX:
            continue
        img.paste(s.image, (x, y), s.image)
    return img



### Top level

def read_obj(filename, load_func):
    with open(filename, 'rb') as f:
        return load_func(f.read())

DIRS = (
        V2( 1, -1),
        V2( 0, -1),
        V2(-1, -1),
        V2(-1,  0),
        V2(-1,  1),
        V2( 0,  1),
        V2( 1,  1),
        V2( 1,  0),
        )

NATURAL_STRUCTURES = set((
    'tree',
    'stump',
    'rock',
    'chest',
    'dungeon_entrance',
))

def collect_player_chunks(save_dir, plane):
    modified = set()

    i = 0
    count = len(plane.saved_chunks)
    for cpos, stable_id in plane.saved_chunks.items():
        path = os.path.join(save_dir, 'terrain_chunks', '%x.terrain_chunk' % stable_id.id)
        chunk = read_obj(path, load_terrain_chunk)
        for s in chunk.child_structures:
            if s.template not in NATURAL_STRUCTURES:
                modified.add(cpos)
                break
        i += 1
        if i % 100 == 0:
            print('  %6d / %6d' % (i, count))

    return modified

def expand_chunks(chunks):
    expanded = set()
    for cpos in chunks:
        expanded.add(cpos)
        for d in DIRS:
            expanded.add(cpos + d)
    return expanded

class SliceCache(object):
    def __init__(self, data, save_dir, plane):
        self.data = data
        self.save_dir = save_dir
        self.plane = plane
        self.cache = {}

    @lru_cache(maxsize=128)
    def get_slices(self, cpos):
        chunk_id = self.plane.saved_chunks.get(cpos, None)
        if chunk_id is None:
            return []

        path = os.path.join(self.save_dir, 'terrain_chunks', '%x.terrain_chunk' % chunk_id.id)
        try:
            chunk = read_obj(path, load_terrain_chunk)
        except IOError:
            return []

        return slice_chunk(self.data, chunk)

# Offsets of all chunks that may contain blocks/structures that overlap the
# center chunk.
OVERLAP_OFFSETS = (
        V2( 0, 0),
        V2( 0, 1),
        V2( 0,-1),
        V2(-1, 0),
        V2(-1, 1),
        V2(-1,-1),
        )

def render_chunk(slice_cache, cpos):
    img = Image.new('RGBA', (CHUNK_PX, CHUNK_PX))
    for offset in OVERLAP_OFFSETS:
        slices = slice_cache.get_slices(cpos + offset)
        ox = offset.x * CHUNK_SIZE
        oy = offset.y * CHUNK_SIZE
        render_slices(img, slices, (ox, oy))
    return img

def render_map(slice_cache, layers, map_dir):
    total_tiles = sum(len(l) for l in layers)
    rendered_tiles = 0
    def count():
        nonlocal rendered_tiles
        rendered_tiles += 1
        if rendered_tiles % 10 == 0:
            print('  %4d / %4d' % (rendered_tiles, total_tiles))

    max_z = len(layers) - 1
    radius2 = 1 << (max_z - 1)
    def render_tile(x, y, z):
        if (x,y) not in layers[z]:
            return None

        if z == max_z:
            img = render_chunk(slice_cache, V2(x, y) - radius2)
            img = img.resize((256, 256), Image.BILINEAR)
        else:
            a = render_tile(x * 2 + 0, y * 2 + 0, z + 1)
            b = render_tile(x * 2 + 1, y * 2 + 0, z + 1)
            c = render_tile(x * 2 + 0, y * 2 + 1, z + 1)
            d = render_tile(x * 2 + 1, y * 2 + 1, z + 1)
            img = Image.new('RGBA', (512, 512))
            if a is not None:
                img.paste(a, (0, 0))
            if b is not None:
                img.paste(b, (256, 0))
            if c is not None:
                img.paste(c, (0, 256))
            if d is not None:
                img.paste(d, (256, 256))
            img = img.resize((256, 256), Image.BILINEAR)

        img.save(os.path.join(map_dir, 'tiles/z%d/%d,%d.png' % (z, x, y)))
        count()
        return img
    render_tile(0, 0, 0)

def main(dist_dir, save_dir, map_dir):
    data = load_data(dist_dir)

    plane = read_obj(os.path.join(save_dir, 'planes/2.plane'), load_plane)
    assert plane.name == 'Everfree Forest', \
            '2.plane has wrong name: %r' % plane.name

    print('scanning for player chunks')
    player_chunks = collect_player_chunks(save_dir, plane)
    print('found %d chunks with player edits' % len(player_chunks))

    expanded_chunks = expand_chunks(player_chunks)
    print('expanded to %d chunks' % len(expanded_chunks))


    radius = max(c + 1 if c >= 0 else -c
            for cpos in expanded_chunks
            for c in (cpos.x, cpos.y))
    rlog2 = math.ceil(math.log2(radius))
    radius2 = 1 << rlog2
    num_layers = 2 + rlog2

    print('computing %d layers' % num_layers)
    layers = [set((cpos.x + radius2, cpos.y + radius2) for cpos in expanded_chunks)]
    for _ in range(num_layers - 1):
        new_layer = set((x // 2, y // 2) for x,y in layers[-1])
        layers.append(new_layer)
    layers.reverse()

    print('rendering %d tiles' % sum(len(l) for l in layers))
    for i in range(num_layers):
        os.makedirs(os.path.join(map_dir, 'tiles/z%d' % i), exist_ok=True)

    slice_cache = SliceCache(data, save_dir, plane)
    render_map(slice_cache, layers, map_dir)


    print('generating map.html')
    with open(os.path.join(os.path.dirname(sys.argv[0]), 'map.tmpl.html'), 'r') as f:
        html = f.read()
    html = html \
            .replace('%DATE%', time.strftime('%Y-%m-%d')) \
            .replace('%MAX_ZOOM%', str(len(layers) - 1)) \
            .replace('%DEFAULT_ZOOM%', str(len(layers) - 3))
    with open(os.path.join(map_dir, 'map.html'), 'w') as f:
        f.write(html)

if __name__ == '__main__':
    dist_dir, save_dir, map_dir = sys.argv[1:]
    main(dist_dir, save_dir, map_dir)
