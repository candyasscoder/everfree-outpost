from itertools import count
import json
import os
import struct
import sys

def main(src_dir, build_dir, out_file):
    index = []
    paths = []

    src = lambda path: os.path.join(src_dir, path)
    build = lambda path: os.path.join(build_dir, path)

    def add(ty, name, path):
        size = os.stat(path).st_size

        index.append({
                'name': name,
                'length': size,
                'type': ty,
                })
        paths.append(path)

    add('image', 'pony_f_base',         src('assets/sprites/maresprite.png'))
    add('image', 'pony_f_eyes_blue',    src('assets/sprites/type1blue.png'))
    add('image', 'pony_f_horn',         src('assets/sprites/marehorn.png'))
    add('image', 'pony_f_wing_front',   src('assets/sprites/frontwingmare.png'))
    add('image', 'pony_f_wing_back',    src('assets/sprites/backwingmare.png'))
    add('image', 'pony_f_mane_1',       src('assets/sprites/maremane1.png'))
    add('image', 'pony_f_tail_1',       src('assets/sprites/maretail1.png'))
    add('image', 'equip_f_hat',         src('assets/sprites/equip_f_hat.png'))

    add('image', 'font',    build('font.png'))
    add('url',   'items',   build('items.png'))

    add('json', 'block_defs',       build('data/blocks_client.json'))
    add('json', 'item_defs',        build('items.json'))
    add('json', 'recipe_defs',      build('recipes.json'))
    add('json', 'template_defs',    build('data/structures_client.json'))
    add('json', 'font_metrics',     build('metrics.json'))
    add('json', 'day_night',        build('day_night.json'))

    add('text', 'terrain.frag',         src('assets/shaders/terrain.frag'))
    add('text', 'terrain.vert',         src('assets/shaders/terrain.vert'))
    add('text', 'sprite.frag',          src('assets/shaders/sprite.frag'))
    add('text', 'sprite.vert',          src('assets/shaders/sprite.vert'))
    add('text', 'sprite_layered.frag',  src('assets/shaders/sprite_layered.frag'))
    add('text', 'sprite_pony_outline.frag', src('assets/shaders/sprite_pony_outline.frag'))
    add('text', 'cursor.frag',          src('assets/shaders/cursor.frag'))
    add('text', 'cursor.vert',          src('assets/shaders/cursor.vert'))

    add('text', 'terrain_block.frag',   src('assets/shaders/terrain_block.frag'))
    add('text', 'terrain_block.vert',   src('assets/shaders/terrain_block.vert'))
    add('text', 'blit.frag',            src('assets/shaders/blit.frag'))
    add('text', 'blit_sliced.frag',     src('assets/shaders/blit_sliced.frag'))
    add('text', 'blit_post.frag',       src('assets/shaders/blit_post.frag'))
    add('text', 'blit_output.frag',     src('assets/shaders/blit_output.frag'))
    add('text', 'blit.vert',            src('assets/shaders/blit.vert'))
    add('text', 'blit_fullscreen.vert', src('assets/shaders/blit_fullscreen.vert'))
    add('text', 'structure.frag',       src('assets/shaders/structure.frag'))
    add('text', 'structure.vert',       src('assets/shaders/structure.vert'))
    add('text', 'light.frag',           src('assets/shaders/light.frag'))
    add('text', 'light.vert',           src('assets/shaders/light.vert'))

    for i in count(0):
        path = build('data/structures%d.png' % i)
        if os.path.isfile(path):
            add('image', 'structures%d' % i, path)
            add('image', 'structdepth%d' % i, build('data/structdepth%d.png' % i))
        else:
            break

    add('image', 'tiles', build('data/tiles.png'))


    # Generate the pack containing the files added above.

    offset = 0
    for entry in index:
        entry['offset'] = offset
        offset += entry['length']


    index_str = json.dumps(index)
    index_len = len(index_str.encode())

    with open(out_file, 'wb') as f:
        f.write(struct.pack('<I', len(index_str.encode())))
        f.write(index_str.encode())

        for (entry, path) in zip(index, paths):
            total_len = 0
            with open(path, 'rb') as f2:
                while True:
                    chunk = f2.read(4096)
                    f.write(chunk)
                    total_len += len(chunk)
                    if len(chunk) == 0:
                        break

            assert total_len == entry['length'], \
                    'file %r changed length during packing' % entry['name']

if __name__ == '__main__':
    src_dir, build_dir, out_file = sys.argv[1:]
    main(src_dir, build_dir, out_file)
