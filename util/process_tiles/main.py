import argparse
import functools
import json
import os
import sys
import yaml

import process_tiles.atlas as A
import process_tiles.blocks as B
import process_tiles.items as I
import process_tiles.objects as O
import process_tiles.recipes as R
import process_tiles.tiles as T
import process_tiles.util as U

def build_parser():
    parser = argparse.ArgumentParser(
            description='Process tile, block, and object data files into a usable form.')

    parser.add_argument('--tile-yaml', metavar='FILE',
            help='YAML file describing the available tiles')
    parser.add_argument('--tile-image-dir', metavar='DIR',
            help='directory containing tile images')

    parser.add_argument('--block-yaml', metavar='FILE',
            help='YAML file describing blocks')
    parser.add_argument('--template-yaml', metavar='FILE',
            help='YAML file describing structure templates')
    parser.add_argument('--item-yaml', metavar='FILE',
            help='YAML file describing items')
    parser.add_argument('--recipe-yaml', metavar='FILE',
            help='YAML file describing recipes')


    parser.add_argument('--block-atlas-image-out', metavar='FILE',
            help='where to write the tile atlas image for blocks')
    parser.add_argument('--item-atlas-image-out', metavar='FILE',
            help='where to write the tile atlas image for items')

    parser.add_argument('--client-block-json-out', metavar='FILE',
            help='where to write the client-side blocks.json')
    parser.add_argument('--server-block-json-out', metavar='FILE',
            help='where to write the server-side blocks.json')
    parser.add_argument('--client-item-json-out', metavar='FILE',
            help='where to write the client-side items.json')
    parser.add_argument('--server-item-json-out', metavar='FILE',
            help='where to write the server-side items.json')
    parser.add_argument('--server-template-json-out', metavar='FILE',
            help='where to write the server-side templates.json')
    parser.add_argument('--client-recipe-json-out', metavar='FILE',
            help='where to write the client-side recipes.json')
    parser.add_argument('--server-recipe-json-out', metavar='FILE',
            help='where to write the server-side recipes.json')

    parser.add_argument('--asset-list-out', metavar='FILE',
            help='where to write the list of used assets')

    return parser

def memoize(f):
    return functools.lru_cache(maxsize=1)(f)

def main():
    parser = build_parser()
    args = parser.parse_args()


    @memoize
    def tile_image_dir():
        if args.tile_image_dir is None:
            parser.error('must provide --tile-image-dir')
        return args.tile_image_dir

    @memoize
    def raw_tiles():
        if args.tile_yaml is None:
            parser.error('must provide --tile-yaml')
        with open(args.tile_yaml) as f:
            return yaml.load(f)

    @memoize
    def raw_blocks():
        if args.block_yaml is None:
            parser.error('must provide --block-yaml')
        with open(args.block_yaml) as f:
            return yaml.load(f)

    @memoize
    def raw_items():
        if args.item_yaml is None:
            parser.error('must provide --item-yaml')
        with open(args.item_yaml) as f:
            return yaml.load(f)

    @memoize
    def raw_objects():
        if args.template_yaml is None:
            parser.error('must provide --template-yaml')
        with open(args.template_yaml) as f:
            return yaml.load(f)

    @memoize
    def raw_recipes():
        if args.recipe_yaml is None:
            parser.error('must provide --recipe-yaml')
        with open(args.recipe_yaml) as f:
            return yaml.load(f)

    tiles = memoize(lambda: T.parse_raw(raw_tiles()))

    blocks = memoize(lambda: B.parse_raw(raw_blocks()))
    block_arr = memoize(lambda: U.build_array(blocks()))

    items = memoize(lambda: I.parse_raw(raw_items()))
    item_arr = memoize(lambda: U.build_array(items()))
    items_by_name = memoize(lambda: U.build_name_map(items()))

    objects = memoize(lambda: O.parse_raw(raw_objects()))
    object_arr = memoize(lambda: U.build_array(objects()))
    objects_by_name = memoize(lambda: U.build_name_map(items()))

    recipes = memoize(lambda: R.parse_raw(raw_recipes()))
    recipe_arr = memoize(lambda: U.build_array(recipes()))


    block_atlas_order_and_lookup = memoize(lambda: A.compute_atlas(block_arr(), tiles(), B.SIDES))
    block_atlas_order = memoize(lambda: block_atlas_order_and_lookup()[0])
    block_atlas_lookup = memoize(lambda: block_atlas_order_and_lookup()[1])
    block_atlas_image = memoize(lambda: A.build_atlas_image(block_atlas_order(), tile_image_dir()))

    item_atlas_order_and_lookup = memoize(lambda: A.compute_atlas(item_arr(), tiles(), ['tile']))
    item_atlas_order = memoize(lambda: item_atlas_order_and_lookup()[0])
    item_atlas_lookup = memoize(lambda: item_atlas_order_and_lookup()[1])
    item_atlas_image = memoize(lambda: A.build_atlas_image(item_atlas_order(), tile_image_dir()))


    did_something = False

    if args.block_atlas_image_out is not None:
        did_something = True
        A.save_image(block_atlas_image(), args.block_atlas_image_out)

    if args.item_atlas_image_out is not None:
        did_something = True
        A.save_image(item_atlas_image(), args.item_atlas_image_out)


    if args.client_block_json_out is not None:
        did_something = True
        j = {
                'blocks': B.build_client_json(block_arr(), block_atlas_lookup()),
                'opaque': A.build_client_json(block_atlas_image()),
                }
        with open(args.client_block_json_out, 'w') as f:
            json.dump(j, f)

    if args.server_block_json_out is not None:
        did_something = True
        j = {
                'blocks': B.build_server_json(block_arr()),
                }
        with open(args.server_block_json_out, 'w') as f:
            json.dump(j, f)


    if args.client_item_json_out is not None:
        did_something = True
        j = {
                'items': I.build_client_json(item_arr(), item_atlas_lookup()),
                }
        with open(args.client_item_json_out, 'w') as f:
            json.dump(j, f)

    if args.server_item_json_out is not None:
        did_something = True
        j = {
                'items': I.build_server_json(item_arr()),
                }
        with open(args.server_item_json_out, 'w') as f:
            json.dump(j, f)


    if args.server_template_json_out is not None:
        did_something = True
        j = {
                'objects': O.build_json(object_arr())
                }
        with open(args.server_template_json_out, 'w') as f:
            json.dump(j, f)


    if args.client_recipe_json_out is not None:
        did_something = True
        j = {
                'recipes': R.build_client_json(recipe_arr(), items_by_name(), objects_by_name()),
                }
        with open(args.client_recipe_json_out, 'w') as f:
            json.dump(j, f)

    if args.server_recipe_json_out is not None:
        did_something = True
        j = {
                'recipes': R.build_server_json(recipe_arr(), items_by_name(), objects_by_name()),
                }
        with open(args.server_recipe_json_out, 'w') as f:
            json.dump(j, f)


    if args.asset_list_out is not None:
        did_something = True
        sheets = A.collect_sheets(block_atlas_order()) | \
                A.collect_sheets(item_atlas_order())
        with open(args.asset_list_out, 'w') as f:
            for sheet in sheets:
                f.write('%s\n' % os.path.join(tile_image_dir(), sheet))


    if not did_something:
        parser.error('must specify at least one output option')

if __name__ == '__main__':
    main()
