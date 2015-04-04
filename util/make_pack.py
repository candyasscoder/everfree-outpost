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

    add('image', 'tiles',   build('tiles.png'))
    add('image', 'font',    build('font.png'))
    add('url',   'items',   build('items.png'))

    add('json', 'tile_defs',        build('tiles.json'))
    add('json', 'item_defs',        build('items.json'))
    add('json', 'recipe_defs',      build('recipes.json'))
    add('json', 'font_metrics',     build('metrics.json'))

    add('text', 'terrain.frag',         src('assets/shaders/terrain.frag'))
    add('text', 'terrain.vert',         src('assets/shaders/terrain.vert'))
    add('text', 'sprite.frag',          src('assets/shaders/sprite.frag'))
    add('text', 'sprite.vert',          src('assets/shaders/sprite.vert'))
    add('text', 'sprite_layered.frag',  src('assets/shaders/sprite_layered.frag'))
    add('text', 'cursor.frag',          src('assets/shaders/cursor.frag'))
    add('text', 'cursor.vert',          src('assets/shaders/cursor.vert'))


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
