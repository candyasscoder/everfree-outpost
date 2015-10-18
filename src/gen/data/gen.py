from collections import namedtuple
import json
import os


from . import builder, builder2, files, loader, util
from . import structure, block, item, recipe, animation, attachment, model, extra
from outpost_data.core.loader import TimeIt


IdMaps = namedtuple('IdMaps', (
    'structures',
    'blocks',
    'items',
    'recipes',
    'animations',
    'attach_slots',
    'attachments_by_slot',
    'models',
))

def copy_builder2_to_builder(b):
    def dump(k, lst):
        for proto in builder2.INSTANCES[k]._dct.values():
            lst.append(proto.instantiate())
    dump('structure', b.structures)
    dump('item', b.items)
    dump('recipe', b.recipes)

def postprocess(b):
    id_maps = IdMaps(
        util.assign_ids(b.structures),
        util.assign_ids(b.blocks, {'empty', 'placeholder'}),
        util.assign_ids(b.items, {'none'}),
        util.assign_ids(b.recipes),
        util.assign_ids(b.animations),
        util.assign_ids(b.attach_slots),
        dict((s.name, util.assign_ids(s.variants, {'none'})) for s in b.attach_slots),
        model.assign_offsets(b.models),
    )

    structure.resolve_model_offsets(b.structures, id_maps.models)
    recipe.resolve_item_ids(b.recipes, id_maps.items)
    recipe.resolve_structure_ids(b.recipes, id_maps.structures)
    extra.resolve_all(b.extras, b, id_maps)

def write_json(output_dir, basename, j):
    with open(os.path.join(output_dir, basename), 'w') as f:
        json.dump(j, f)

def emit_structures(output_dir, structures):
    for f in os.listdir(output_dir):
        if (f.startswith('structures') or f.startswith('structdepth')) and f.endswith('.png'):
            os.remove(os.path.join(output_dir, f))

    sheet_names = set()
    sheets = structure.build_sheets(structures)
    for i, image in enumerate(sheets):
        sheet_names.update(('structures%d' % i,))
        image.save(os.path.join(output_dir, 'structures%d.png' % i))

    anim_sheets = structure.build_anim_sheets(structures)
    for i, image in enumerate(anim_sheets):
        sheet_names.update(('staticanim%d' % i,))
        image.save(os.path.join(output_dir, 'staticanim%d.png' % i))

    write_json(output_dir, 'structures_server.json',
            structure.build_server_json(structures))

    write_json(output_dir, 'structures_client.json',
            structure.build_client_json(structures))

    write_json(output_dir, 'structures_list.json', sorted(sheet_names))

def emit_blocks(output_dir, blocks):
    sheet = block.build_sheet(blocks)
    sheet.save(os.path.join(output_dir, 'tiles.png'))

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

        for i, img in enumerate(s.images):
            basename = '%s-%d' % (s.name.replace('/', '_'), i)
            sprite_names.add(basename)
            img.save(os.path.join(output_dir, 'sprites', basename + '.png'))

    write_json(output_dir, 'sprites_list.json', sorted(sprite_names))

def emit_attach_slots(output_dir, attach_slots):
    write_json(output_dir, 'attach_slots_server.json',
            attachment.build_server_json(attach_slots))

    write_json(output_dir, 'attach_slots_client.json',
            attachment.build_client_json(attach_slots))

def emit_models(output_dir, models):
    write_json(output_dir, 'models_client.json',
            model.build_client_json(models))

def emit_extras(output_dir, extras):
    write_json(output_dir, 'extras_client.json',
            extra.build_client_json(extras))

def time(msg, f, *args):
    with TimeIt('  %s' % msg):
        f(*args)

def generate(output_dir):
    b = builder.INSTANCE
    copy_builder2_to_builder(b)
    postprocess(b)

    print('Generating:')
    time('structures', emit_structures, output_dir, b.structures)
    time('blocks', emit_blocks, output_dir, b.blocks)
    time('items', emit_items, output_dir, b.items)
    time('recipes', emit_recipes, output_dir, b.recipes)
    time('animations', emit_animations, output_dir, b.animations)
    time('sprites', emit_sprites, output_dir, b.sprites)
    time('attach_slots', emit_attach_slots, output_dir, b.attach_slots)
    time('models', emit_models, output_dir, b.models)
    time('extras', emit_extras, output_dir, b.extras)

    print('%d structures, %d blocks, %d items, %d recipes' %
            (len(b.structures), len(b.blocks), len(b.items), len(b.recipes)))
    print('%d animations, %d sprites, %d attach_slots, %d models, %d extras' %
            (len(b.animations), len(b.sprites), len(b.attach_slots), len(b.models), len(b.extras)))

    with open(os.path.join(output_dir, 'stamp'), 'w') as f:
        pass

    with open(os.path.join(output_dir, 'used_assets.txt'), 'w') as f:
        f.write(''.join(p + '\n' for p in files.get_dependencies()))

    # Compute dependencies
    with open(os.path.join(output_dir, 'data.d'), 'w') as f:
        f.write('%s: \\\n' % os.path.join(output_dir, 'stamp'))
        for p in files.get_dependencies() + loader.get_dependencies():
            f.write('    %s \\\n' % p)

    assert not util.SAW_ERROR

