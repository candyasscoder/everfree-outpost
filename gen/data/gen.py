import json
import os


from . import builder, images, loader, util
from . import structure as S
from . import tile as T
from . import block as B
from . import item as I
from . import recipe as R


def postprocess(b):
    structure_id_map = util.assign_ids(b.structures)
    tile_id_map = util.assign_ids(b.tiles, {'empty'})
    block_id_map = util.assign_ids(b.blocks, {'empty'})
    item_id_map = util.assign_ids(b.items, {'none'})
    recipe_id_map = util.assign_ids(b.recipes)

    B.resolve_tile_ids(b.blocks, tile_id_map)
    R.resolve_item_ids(b.recipes, item_id_map)
    R.resolve_structure_ids(b.recipes, structure_id_map)

def write_json(output_dir, basename, j):
    with open(os.path.join(output_dir, basename), 'w') as f:
        json.dump(j, f)

def emit_structures(output_dir, structures):
    sheets = S.build_sheets(structures)
    for i, (image, depthmap) in enumerate(sheets):
        image.save(os.path.join(output_dir, 'structures%d.png' % i))
        depthmap.save(os.path.join(output_dir, 'structdepth%d.png' % i))

    write_json(output_dir, 'structures_server.json',
            S.build_server_json(structures))

    write_json(output_dir, 'structures_client.json',
            S.build_client_json(structures))

def emit_tiles(output_dir, tiles):
    sheet = util.build_sheet(tiles)
    sheet.save(os.path.join(output_dir, 'tiles.png'))

def emit_blocks(output_dir, blocks):
    write_json(output_dir, 'blocks_server.json',
            B.build_server_json(blocks))

    write_json(output_dir, 'blocks_client.json',
            B.build_client_json(blocks))

def emit_items(output_dir, items):
    sheet = util.build_sheet(items)
    sheet.save(os.path.join(output_dir, 'items.png'))

    write_json(output_dir, 'items_server.json',
            I.build_server_json(items))

    write_json(output_dir, 'items_client.json',
            I.build_client_json(items))

def emit_recipes(output_dir, recipes):
    write_json(output_dir, 'recipes_server.json',
            R.build_server_json(recipes))

    write_json(output_dir, 'recipes_client.json',
            R.build_client_json(recipes))

def generate(output_dir):
    b = builder.INSTANCE
    postprocess(b)

    emit_structures(output_dir, b.structures)
    emit_tiles(output_dir, b.tiles)
    emit_blocks(output_dir, b.blocks)
    emit_items(output_dir, b.items)
    emit_recipes(output_dir, b.recipes)

    with open(os.path.join(output_dir, 'used_assets.txt'), 'w') as f:
        f.write(''.join(p + '\n' for p in images.DEPENDENCIES))

    # Compute dependencies
    with open(os.path.join(output_dir, 'data.d'), 'w') as f:
        f.write('%s: \\\n' % os.path.join(output_dir, 'stamp'))
        for p in images.get_dependencies() + loader.get_dependencies():
            f.write('    %s \\\n' % p)

    assert not util.SAW_ERROR

