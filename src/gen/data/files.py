from collections import namedtuple
import os

from . import util

ModInfo = namedtuple('ModInfo', ('assets', 'overrides', 'deps'))
MOD_MAP = {}

def register_mod(name, assets, override_dir, deps):
    # NB: `override_dir` is a directory containing overrides for *other*,
    # previously loaded mods.  The `overrides` field of `ModInfo` is a list of
    # override directories applied to *this* mod by others.
    if name in MOD_MAP:
        util.err('mod %r is loaded multiple times' % name)

    for dep in deps:
        if dep not in MOD_MAP:
            util.err('mod %r depends on %r, which is not (yet) loaded' % (name, dep))

    MOD_MAP[name] = ModInfo(assets, [], deps)

    if override_dir is not None and os.path.exists(override_dir):
        for override_mod in os.listdir(override_dir):
            if override_mod not in MOD_MAP:
                util.err('mod %r applies overrides to %r, which is not (yet) loaded' %
                        (name, override_mod))
                continue

            MOD_MAP[override_mod].overrides.append(os.path.join(override_dir, override_mod))


DEPENDENCIES = set()

FIND_CACHE = {}

def _find_file_in_dir(dir_path, path):
    full_path = os.path.join(dir_path, path)
    DEPENDENCIES.add(os.path.dirname(full_path))
    if os.path.exists(full_path):
        DEPENDENCIES.add(full_path)
        return full_path
    else:
        return None

def _find_file(mod_name, path):
    key = (mod_name, path)
    if key not in FIND_CACHE:
        FIND_CACHE[key] = _find_file_uncached(mod_name, path)
    return FIND_CACHE[key]

def _find_file_uncached(mod_name, path):
    """Find the full path of the file `path` in `mod`.  Returns `None` on failure."""
    mod = MOD_MAP[mod_name]
    for override in reversed(mod.overrides):
        img = _find_file_in_dir(override, path)
        if img is not None:
            return img

    img = _find_file_in_dir(mod.assets, path)
    if img is not None:
        return img

    for dep in mod.deps:
        img = _find_file(dep, path)
        if img is not None:
            return img

    return None

def find(path, mod=None):
    mod = mod or util.get_caller_mod_name()
    return _find_file(mod, path)

def get_dependencies():
    return list(DEPENDENCIES)
