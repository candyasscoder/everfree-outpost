import argparse
from collections import namedtuple
from functools import lru_cache
import os
import sys
import time

import tornado.autoreload
import tornado.web
from PIL import Image

from outpost_savegame import V2, V3
try:
    import outpost_terrain_gen as tg
except ImportError:
    # Sometimes the import fails because the autoreload triggered too early,
    # before the linker finished writing the .so.
    time.sleep(1.0)
    import outpost_terrain_gen as tg
import render_map


def int_pair(s):
    parts = s.split(',')
    if len(parts) != 2:
        raise ValueError('expected 2 items, but found %d: %r' % (len(parts), s))
    return (int(parts[0]), int(parts[1]))

def build_parser():
    p = argparse.ArgumentParser()
    p.add_argument('dist_dir', metavar='DIST_DIR',
            help='path to the outpost distribution')
    p.add_argument('--plane-id', metavar='INT', type=int, default=2,
            help='stable ID of the plane to generate')
    p.add_argument('--port', metavar='PORT', type=int, default=8000,
            help='port number for running the HTTP server (default: 8000)')
    p.add_argument('--origin', metavar='X,Y', type=int_pair, default=(0, 0),
            help='center point for map generation (default: 0,0)')
    return p


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
    def __init__(self, data, get_chunk):
        self.data = data
        self.get_chunk = get_chunk

    @lru_cache(maxsize=128)
    def get_slices(self, cpos):
        chunk = self.get_chunk(cpos)
        if chunk is None:
            return []

        return render_map.slice_chunk(self.data, chunk)


def mk_map_handler():
    with open(os.path.join(os.path.dirname(sys.argv[0]), 'map.tmpl.html'), 'r') as f:
        html = f.read()
        html = html \
                .replace('%DATE%', 'Terrain Generation Test') \
                .replace('%MAX_ZOOM%', '10') \
                .replace('%DEFAULT_ZOOM%', '10')

    class MapHandler(tornado.web.RequestHandler):
        def get(self):
            self.write(html)

    return MapHandler

ZOOM = 10
CENTER_TILE = 1 << (ZOOM - 1)

def mk_tile_handler(args):
    offset = V2(*args.origin) - V2(CENTER_TILE, CENTER_TILE)

    data = render_map.load_data(args.dist_dir)
    worker = tg.Worker(args.dist_dir)

    wrap_chunk = mk_wrap(data)
    def get_chunk(cpos):
        worker.request(args.plane_id, cpos.x, cpos.y)
        _, _, _, chunk = worker.get_response()
        return wrap_chunk(chunk)
    slice_cache = SliceCache(data, get_chunk)

    @lru_cache(maxsize=256)
    def generate_image(tile_pos, z):
        print('generate %d, %d, %d' % (tile_pos.x, tile_pos.y, z))
        if z == ZOOM:
            cpos = tile_pos + offset
            img = render_map.render_chunk(slice_cache, cpos)
            return img.resize((256, 256))
        else:
            parts = [(x, y, generate_image(tile_pos * 2 + V2(x, y), z + 1))
                    for x in (0, 1) for y in (0, 1)]
            img = Image.new('RGBA', (512, 512))
            for x, y, subimg in parts:
                img.paste(subimg, (x * 256, y * 256))
            return img.resize((256, 256))


    class TileHandler(tornado.web.RequestHandler):

        def get(self, z_str, x_str, y_str):
            tile_pos = V2(int(x_str), int(y_str))
            img = generate_image(tile_pos, int(z_str))
            img.save(self, format='PNG')

    return TileHandler


def mk_application(args):
    MapHandler = mk_map_handler()
    TileHandler = mk_tile_handler(args)

    return tornado.web.Application([
        (r'/', MapHandler),
        (r'/map.html', MapHandler),
        (r'/tiles/z([0-9]*)/([0-9]*),([0-9]*).png', TileHandler),
    ], debug=True)

def main(argv):
    args = build_parser().parse_args(argv)
    tornado.autoreload.watch(tg.__file__)

    application = mk_application(args)
    application.listen(args.port)
    tornado.ioloop.IOLoop.instance().start()

if __name__ == '__main__':
    main(sys.argv[1:])
