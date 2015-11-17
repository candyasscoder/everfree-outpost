from itertools import count
import json
import os
import struct
import sys

def main(src_dir, build_dir, out_file):
    index = []
    paths = []
    hidden_deps = set()

    src = lambda path: os.path.join(src_dir, path)
    build = lambda path: os.path.join(build_dir, path)

    def add(ty, name, path, hide_dep=False):
        size = os.stat(path).st_size

        index.append({
                'name': name,
                'length': size,
                'type': ty,
                })
        paths.append(path)

        if hide_dep:
            hidden_deps.add(path)

    add('image', 'font',    build('font.png'))
    add('url',   'items',   build('items.png'))

    add('json', 'block_defs',       build('blocks_client.json'))
    add('json', 'item_defs',        build('items_client.json'))
    add('json', 'recipe_defs',      build('recipes_client.json'))
    add('json', 'template_defs',    build('structures_client.json'))
    add('json', 'animation_defs',   build('animations_client.json'))
    add('json', 'attach_slot_defs', build('attach_slots_client.json'))
    add('json', 'model_defs',       build('models_client.json'))
    add('json', 'extra_defs',       build('extras_client.json'))
    add('json', 'font_metrics',     build('font_metrics.json'))
    add('json', 'day_night',        build('day_night.json'))

    add('text', 'sprite.vert',          src('assets/shaders/sprite.vert'))
    add('text', 'sprite.frag',          src('assets/shaders/sprite.frag'))
    add('text', 'app_pony.frag',        src('assets/shaders/app_pony.frag'))
    add('text', 'cursor.frag',          src('assets/shaders/cursor.frag'))
    add('text', 'cursor.vert',          src('assets/shaders/cursor.vert'))

    add('text', 'blit.frag',            src('assets/shaders/blit.frag'))
    add('text', 'blit_sliced.frag',     src('assets/shaders/blit_sliced.frag'))
    add('text', 'blit_post.frag',       src('assets/shaders/blit_post.frag'))
    add('text', 'blit_output.frag',     src('assets/shaders/blit_output.frag'))
    add('text', 'blit_depth.frag',      src('assets/shaders/blit_depth.frag'))
    add('text', 'blit.vert',            src('assets/shaders/blit.vert'))
    add('text', 'blit_fullscreen.vert', src('assets/shaders/blit_fullscreen.vert'))

    add('text', 'terrain2.frag',        src('assets/shaders/terrain2.frag'))
    add('text', 'terrain2.vert',        src('assets/shaders/terrain2.vert'))
    add('text', 'structure2.frag',      src('assets/shaders/structure2.frag'))
    add('text', 'structure2.vert',      src('assets/shaders/structure2.vert'))
    add('text', 'light2.frag',          src('assets/shaders/light2.frag'))
    add('text', 'light2.vert',          src('assets/shaders/light2.vert'))

    add('image', 'tiles', build('tiles.png'))

    with open(build('structures_list.json')) as f:
        sprites_list = json.load(f)
    for s in sprites_list:
        add('image', s, build(s + '.png'))

    with open(build('sprites_list.json')) as f:
        sprites_list = json.load(f)
    for s in sprites_list:
        add('image', s, build(os.path.join('sprites', s + '.png')))


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

    # Emit dependencies
    with open(out_file + '.d', 'w') as f:
        f.write('%s: \\\n' % out_file)
        for path in paths:
            if path in hidden_deps:
                continue
            f.write('    %s \\\n' % path)

if __name__ == '__main__':
    src_dir, build_dir, out_file = sys.argv[1:]
    main(src_dir, build_dir, out_file)
