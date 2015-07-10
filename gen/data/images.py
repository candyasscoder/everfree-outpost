import os

from PIL import Image


IMAGE_CACHE = {}
DEPENDENCIES = set()
SEARCH_PATH = ()

def load(path):
    if path not in IMAGE_CACHE:
        for asset_dir in SEARCH_PATH:
            full_path = os.path.join(asset_dir, path)

            # If files are added or removed along the relevant search path, we
            # may need to rebuild.
            DEPENDENCIES.add(os.path.dirname(full_path))

            if os.path.isfile(full_path):
                IMAGE_CACHE[path] = Image.open(full_path)
                DEPENDENCIES.add(full_path)
                break
        else:
            # Not found
            raise KeyError('could not find asset %r' % path)
    return IMAGE_CACHE[path]

def loader(path):
    return lambda n: load(os.path.join(path, n))

def get_dependencies():
    return list(DEPENDENCIES)
