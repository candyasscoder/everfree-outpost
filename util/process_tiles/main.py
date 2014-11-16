import argparse
import json
import sys
import yaml

import process_tiles.atlas as A
import process_tiles.blocks as B
import process_tiles.tiles as T

def build_parser():
    parser = argparse.ArgumentParser(
            description='Process blocks and tiles into a usable form.')
    parser.add_argument('block_yaml', metavar='BLOCK_YAML',
            help='YAML file describing the available blocks')

    parser.add_argument('--tile-yaml', metavar='FILE',
            help='YAML file describing the available tiles')
    parser.add_argument('--tile-image-dir', metavar='DIR',
            help='directory containing tile images')
    parser.add_argument('--atlas-image-out', metavar='FILE',
            help='where to write the tile atlas image')
    parser.add_argument('--client-json-out', metavar='FILE',
            help='where to write the client-side blocks.json')
    parser.add_argument('--server-json-out', metavar='FILE',
            help='where to write the server-side blocks.json')

    return parser

def main():
    parser = build_parser()
    args = parser.parse_args()

    if (args.atlas_image_out is not None or args.client_json_out is not None) and \
            (args.tile_yaml is None or args.tile_image_dir is None):
        which = '--client-json-out' if args.client_json_out is not None else '--atlas-image-out'
        parser.error('%s requires --tile-yaml and --tile-image-dir to be set' % which)

    if args.atlas_image_out is None and args.client_json_out is None and \
            args.server_json_out is None:
        parser.error('must specify at least one of '
                '--atlas-image-out, --client-json-out, and --server-json-out')

    with open(args.block_yaml) as f:
        raw_blocks = yaml.load(f)
    blocks = B.parse_raw(raw_blocks)
    block_arr = B.build_array(blocks)

    if args.client_json_out is not None or args.atlas_image_out is not None:
        with open(args.tile_yaml) as f:
            raw_tiles = yaml.load(f)
        tiles = T.parse_raw(raw_tiles)

        atlas_order, atlas_lookup = A.compute_atlas(block_arr, tiles)
        atlas_image = A.build_atlas_image(atlas_order, args.tile_image_dir)


    if args.atlas_image_out is not None:
        A.save_image(atlas_image, args.atlas_image_out)

    if args.client_json_out is not None:
        j = {
                'blocks': B.build_client_json(block_arr, atlas_lookup),
                'opaque': A.build_client_json(atlas_image),
                }
        with open(args.client_json_out, 'w') as f:
            json.dump(j, f)

    if args.server_json_out is not None:
        with open(args.server_json_out, 'w') as f:
            j = {
                    'blocks': B.build_server_json(block_arr),
                    }
            json.dump(j, f)

if __name__ == '__main__':
    main()
