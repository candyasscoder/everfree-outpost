import argparse
import functools
import json
import os
import sys
import yaml

import process_tiles.atlas as A
import process_tiles.blocks as B
import process_tiles.objects as O
import process_tiles.tiles as T
import process_tiles.util as U

def build_parser():
    parser = argparse.ArgumentParser(
            description='Process tile, block, and object data files into a usable form.')

    parser.add_argument('--block-yaml', metavar='FILE',
            help='YAML file describing the available blocks')
    parser.add_argument('--tile-yaml', metavar='FILE',
            help='YAML file describing the available tiles')
    parser.add_argument('--object-yaml', metavar='FILE',
            help='YAML file describing object templates')
    parser.add_argument('--tile-image-dir', metavar='DIR',
            help='directory containing tile images')

    parser.add_argument('--atlas-image-out', metavar='FILE',
            help='where to write the tile atlas image')
    parser.add_argument('--client-json-out', metavar='FILE',
            help='where to write the client-side blocks.json')
    parser.add_argument('--server-json-out', metavar='FILE',
            help='where to write the server-side blocks.json')
    parser.add_argument('--object-json-out', metavar='FILE',
            help='where to write the server-side objects.json')
    parser.add_argument('--asset-list-out', metavar='FILE',
            help='where to write the list of used assets')

    return parser

def memoize(f):
    return functools.lru_cache(maxsize=1)(f)

def main():
    parser = build_parser()
    args = parser.parse_args()

    if (args.atlas_image_out is not None or args.client_json_out is not None) and \
            (args.tile_yaml is None or args.tile_image_dir is None):
        which = '--client-json-out' if args.client_json_out is not None else '--atlas-image-out'
        parser.error('%s requires --tile-yaml and --tile-image-dir to be set' % which)

    if args.object_json_out is not None and args.object_yaml is None:
        parser.error('--object-json-out requires --object-yaml to be set')


    @memoize
    def raw_blocks():
        if args.block_yaml is None:
            parser.error('must provide --block-yaml')
        with open(args.block_yaml) as f:
            return yaml.load(f)

    @memoize
    def raw_tiles():
        if args.tile_yaml is None:
            parser.error('must provide --tile-yaml')
        with open(args.tile_yaml) as f:
            return yaml.load(f)

    @memoize
    def raw_objects():
        if args.object_yaml is None:
            parser.error('must provide --object-yaml')
        with open(args.object_yaml) as f:
            return yaml.load(f)

    @memoize
    def tile_image_dir():
        if args.tile_image_dir is None:
            parser.error('must provide --tile-image-dir')
        return args.tile_image_dir

    blocks = memoize(lambda: B.parse_raw(raw_blocks()))
    block_arr = memoize(lambda: U.build_array(blocks()))

    tiles = memoize(lambda: T.parse_raw(raw_tiles()))

    atlas_order_and_lookup = memoize(lambda: A.compute_atlas(block_arr(), tiles()))
    atlas_order = memoize(lambda: atlas_order_and_lookup()[0])
    atlas_lookup = memoize(lambda: atlas_order_and_lookup()[1])
    atlas_image = memoize(lambda: A.build_atlas_image(atlas_order(), tile_image_dir()))

    objects = memoize(lambda: O.parse_raw(raw_objects()))
    object_arr = memoize(lambda: U.build_array(objects()))


    did_something = False

    if args.atlas_image_out is not None:
        did_something = True
        A.save_image(atlas_image(), args.atlas_image_out)

    if args.client_json_out is not None:
        did_something = True
        j = {
                'blocks': B.build_client_json(block_arr(), atlas_lookup()),
                'opaque': A.build_client_json(atlas_image()),
                }
        with open(args.client_json_out, 'w') as f:
            json.dump(j, f)

    if args.server_json_out is not None:
        did_something = True
        j = {
                'blocks': B.build_server_json(block_arr()),
                }
        with open(args.server_json_out, 'w') as f:
            json.dump(j, f)

    if args.object_json_out is not None:
        did_something = True
        j = {
                'objects': O.build_json(object_arr())
                }
        with open(args.object_json_out, 'w') as f:
            json.dump(j, f)

    if args.asset_list_out is not None:
        did_something = True
        sheets = A.collect_sheets(atlas_order())
        with open(args.asset_list_out, 'w') as f:
            for sheet in sheets:
                f.write('%s\n' % os.path.join(tile_image_dir(), sheet))


    if not did_something:
        parser.error('must specify at least one output option')

if __name__ == '__main__':
    main()
