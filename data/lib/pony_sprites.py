from ...core.builder import *
from ...core.images import loader
from ...core.util import cached, err

from PIL import Image


DIRS = [
        {'idx': 2},
        {'idx': 3},
        {'idx': 4},
        {'idx': 3, 'mirror': 1},
        {'idx': 2, 'mirror': 0},
        {'idx': 1, 'mirror': 7},
        {'idx': 0},
        {'idx': 1},
        ]

MOTIONS = [
        {'name': 'stand', 'row': 0, 'base_col': 0, 'len': 1, 'fps': 1},
        {'name': 'walk', 'row': 1, 'base_col': 0, 'len': 6, 'fps': 8},
        {'name': 'run', 'row': 3, 'base_col': 0, 'len': 6, 'fps': 12},
        ]

@cached
def get_anim_group():
    g = mk_anim_group('pony')

    for m in MOTIONS:
        for i, d in enumerate(DIRS):
            mirror = d.get('mirror')
            if mirror is None:
                g.add_anim('pony/%s-%d' % (m['name'], i), m['len'], m['fps'])
            else:
                g.add_anim_mirror('pony/%s-%d' % (m['name'], i),
                        'pony/%s-%d' % (m['name'], mirror))
    g.finish()
    return g.unwrap()
