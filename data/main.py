import json
import os
import sys

import outpost_data.images as I
import outpost_data.structure as S
from structures import get_structures

def main(asset_dir, output_dir):
    structures = get_structures(asset_dir)
    sheets = S.build_sheets(structures)

    for i, (image, depthmap) in enumerate(sheets):
        image.save(os.path.join(output_dir, 'structures%d.png' % i))
        depthmap.save(os.path.join(output_dir, 'structdepth%d.png' % i))

    with open(os.path.join(output_dir, 'structures_server.json'), 'w') as f:
        j = S.build_server_json(structures)
        json.dump(j, f)

    with open(os.path.join(output_dir, 'structures_client.json'), 'w') as f:
        j = S.build_client_json(structures)
        json.dump(j, f)

    with open(os.path.join(output_dir, 'used_assets.txt'), 'w') as f:
        f.write(''.join(p + '\n' for p in I.get_loaded_paths()))

if __name__ == '__main__':
    asset_dir, output_dir = sys.argv[1:]
    main(asset_dir, output_dir)
