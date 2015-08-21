from collections import namedtuple
from functools import lru_cache
import os
import sys

from outpost_savegame import V2, V3
import outpost_terrain_gen as tg
import render_map

ChunkWrapper = namedtuple('ChunkWrapper', ('blocks', 'child_structures'))
StructureWrapper = namedtuple('StructureWrapper', ('offset', 'template'))

def mk_wrap(data):
    template_map = dict((v.id, k) for k,v in data.templates.items())
    block_map = dict((v.id, k) for k,v in data.blocks.items())

    def wrap_structure(s):
        offset = V3(s.x, s.y, s.z)
        template = template_map[s.template]
        return StructureWrapper(offset, template)

    def wrap_chunk(c):
        blocks = [block_map[i] for i in c.blocks]
        child_structures = [wrap_structure(s) for s in c.structures]
        return ChunkWrapper(blocks, child_structures)

    return wrap_chunk

class SliceCache(object):
    def __init__(self, data, chunks):
        self.data = data
        self.chunks = chunks

    @lru_cache(maxsize=128)
    def get_slices(self, cpos):
        if cpos not in self.chunks:
            return []

        c = render_map.slice_chunk(self.data, self.chunks[cpos])
        return c

def main(dist_dir, map_dir, base_x, base_y):
    data = render_map.load_data(dist_dir)
    worker = tg.Worker(dist_dir)

    # Generate a bunch of chunks.
    print(' * Generating chunks')
    chunks = {}
    for y in range(-4, 4):
        for x in range(-4, 4):
            worker.request(2, x + base_x, y + base_y)
            chunks[V2(x, y)] = None

    for i in range(len(chunks)):
        _, x, y, chunk = worker.get_response()
        chunks[V2(x - base_x, y - base_y)] = chunk

    # Wrap all chunks and build the slice cache.
    wrap_chunk = mk_wrap(data)
    wrapped_chunks = dict((k, wrap_chunk(v)) for k,v in chunks.items())
    slice_cache = SliceCache(data, wrapped_chunks)

    # Render all the chunks.
    print(' * Rendering %d chunks' % len(chunks))
    imgs = dict((k, render_map.render_chunk(slice_cache, k)) for k in chunks.keys())

    # Populate map directory.
    ZOOM = 10
    tile_dir = os.path.join(map_dir, 'tiles', 'z%d' % ZOOM)
    os.makedirs(tile_dir, exist_ok=True)

    print(' * Saving tiles')
    tile_base = 1 << (ZOOM - 1)
    for k,v in imgs.items():
        path = os.path.join(tile_dir, '%s,%s.png' % (k.x + tile_base, k.y + tile_base))
        v.resize((256, 256)).save(path)

    with open(os.path.join(os.path.dirname(sys.argv[0]), 'map.tmpl.html'), 'r') as f:
        html = f.read()
    html = html \
            .replace('%DATE%', 'Terrain Generation Test') \
            .replace('%MAX_ZOOM%', '10') \
            .replace('%DEFAULT_ZOOM%', '10')
    with open(os.path.join(map_dir, 'map.html'), 'w') as f:
        f.write(html)

if __name__ == '__main__':
    dist_dir, map_dir = sys.argv[1:3]
    if len(sys.argv) > 3:
        base_x, base_y = sys.argv[3:]
    else:
        base_x, base_y = 0, 0
    main(dist_dir, map_dir, int(base_x), int(base_y))
