import json
import os


from . import builder, images, loader, util
from . import structure, tile, block, item, recipe, animation, attachment


def postprocess(b):
    structure_id_map = util.assign_ids(b.structures)
    tile_id_map = util.assign_ids(b.tiles, {'empty'})
    block_id_map = util.assign_ids(b.blocks, {'empty'})
    item_id_map = util.assign_ids(b.items, {'none'})
    recipe_id_map = util.assign_ids(b.recipes)
    anim_id_map = util.assign_ids(b.animations)
    attach_slot_id_map = util.assign_ids(b.attach_slots)
    for s in b.attach_slots:
        util.assign_ids(s.variants)

    block.resolve_tile_ids(b.blocks, tile_id_map)
    recipe.resolve_item_ids(b.recipes, item_id_map)
    recipe.resolve_structure_ids(b.recipes, structure_id_map)

def write_json(output_dir, basename, j):
    with open(os.path.join(output_dir, basename), 'w') as f:
        json.dump(j, f)

def emit_structures(output_dir, structures):
    for f in os.listdir(output_dir):
        if (f.startswith('structures') or f.startswith('structdepth')) and f.endswith('.png'):
            os.remove(os.path.join(output_dir, f))

    sheets = structure.build_sheets(structures)
    for i, (image, depthmap) in enumerate(sheets):
        image.save(os.path.join(output_dir, 'structures%d.png' % i))
        depthmap.save(os.path.join(output_dir, 'structdepth%d.png' % i))

    write_json(output_dir, 'structures_server.json',
            structure.build_server_json(structures))

    write_json(output_dir, 'structures_client.json',
            structure.build_client_json(structures))

def emit_tiles(output_dir, tiles):
    sheet = util.build_sheet(tiles)
    sheet.save(os.path.join(output_dir, 'tiles.png'))

def emit_blocks(output_dir, blocks):
    write_json(output_dir, 'blocks_server.json',
            block.build_server_json(blocks))

    write_json(output_dir, 'blocks_client.json',
            block.build_client_json(blocks))

def emit_items(output_dir, items):
    sheet = util.build_sheet(items)
    sheet.save(os.path.join(output_dir, 'items.png'))

    write_json(output_dir, 'items_server.json',
            item.build_server_json(items))

    write_json(output_dir, 'items_client.json',
            item.build_client_json(items))

def emit_recipes(output_dir, recipes):
    write_json(output_dir, 'recipes_server.json',
            recipe.build_server_json(recipes))

    write_json(output_dir, 'recipes_client.json',
            recipe.build_client_json(recipes))

def emit_animations(output_dir, animations):
    write_json(output_dir, 'animations_server.json',
            animation.build_server_json(animations))

    write_json(output_dir, 'animations_client.json',
            animation.build_client_json(animations))

def emit_sprites(output_dir, sprites):
    os.makedirs(os.path.join(output_dir, 'sprites'), exist_ok=True)

    sprite_names = set()
    for s in sprites:
        if s.name in sprite_names:
            util.err('duplicate sprite definition: %r' % s.name)
        sprite_names.add(s.name)

        for i, img in enumerate(s.images):
            basename = '%s-%d.png' % (s.name.replace('/', '_'), i)
            img.save(os.path.join(output_dir, 'sprites', basename))

def emit_attach_slots(output_dir, attach_slots):
    write_json(output_dir, 'attach_slots_server.json',
            attachment.build_server_json(attach_slots))

    write_json(output_dir, 'attach_slots_client.json',
            attachment.build_client_json(attach_slots))

def generate(output_dir):
    b = builder.INSTANCE
    postprocess(b)

    emit_structures(output_dir, b.structures)
    emit_tiles(output_dir, b.tiles)
    emit_blocks(output_dir, b.blocks)
    emit_items(output_dir, b.items)
    emit_recipes(output_dir, b.recipes)
    emit_animations(output_dir, b.animations)
    emit_sprites(output_dir, b.sprites)
    emit_attach_slots(output_dir, b.attach_slots)

    with open(os.path.join(output_dir, 'stamp'), 'w') as f:
        pass

    with open(os.path.join(output_dir, 'used_assets.txt'), 'w') as f:
        f.write(''.join(p + '\n' for p in images.get_dependencies()))

    # Compute dependencies
    with open(os.path.join(output_dir, 'data.d'), 'w') as f:
        f.write('%s: \\\n' % os.path.join(output_dir, 'stamp'))
        for p in images.get_dependencies() + loader.get_dependencies():
            f.write('    %s \\\n' % p)

    assert not util.SAW_ERROR

