from collections import namedtuple
import os

from PIL import Image

from . import files
from . import util


# Cache of images indexed by their real file path.
LOAD_CACHE = {}

def _load_image(full_path):
    if full_path not in LOAD_CACHE:
        LOAD_CACHE[full_path] = Image.open(full_path)
    return LOAD_CACHE[full_path]

def load(path, mod=None):
    mod = mod or util.get_caller_mod_name()
    full_path = files.find(path, mod)
    if full_path is None:
        raise KeyError('image not found: %r (in mod %r)' % (path, mod))
    return _load_image(full_path)

def loader(path, mod=None):
    if mod is None:
        mod = util.get_caller_mod_name()
    return lambda n: load(os.path.join(path, n), mod)
