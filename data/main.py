import json
import os
import sys

from outpost_data import images, builder, util
import outpost_data.structure as S
import outpost_data.tile as T
import outpost_data.block as B

import structures
import terrain

def postprocess(b):
    structure_id_map = util.assign_ids(b.structures)
    tile_id_map = util.assign_ids(b.tiles, {'empty'})
    block_id_map = util.assign_ids(b.blocks, {'empty'})

    B.resolve_tile_ids(b.blocks, tile_id_map)

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
    sheet = T.build_sheet(tiles)
    sheet.save(os.path.join(output_dir, 'tiles.png'))

def emit_blocks(output_dir, blocks):
    write_json(output_dir, 'blocks_server.json',
            B.build_server_json(blocks))

    write_json(output_dir, 'blocks_client.json',
            B.build_client_json(blocks))

def main(asset_dir, output_dir):
    terrain.init(asset_dir)
    structures.init(asset_dir)
    b = builder.INSTANCE
    postprocess(b)

    emit_structures(output_dir, b.structures)
    emit_tiles(output_dir, b.tiles)
    emit_blocks(output_dir, b.blocks)

    with open(os.path.join(output_dir, 'used_assets.txt'), 'w') as f:
        f.write(''.join(p + '\n' for p in images.get_loaded_paths()))

    assert not util.SAW_ERROR

if __name__ == '__main__':
    asset_dir, output_dir = sys.argv[1:]
    main(asset_dir, output_dir)
