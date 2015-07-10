from ..core.builder import *
from ..core.images import loader
from ..core import depthmap
from ..core.structure import Shape
from ..core.util import extract

from .lib.items import *
from .lib.structures import *


def do_fence_parts(basename, image):
    fence_parts = (
            ('end/w',           'edge/horiz',       'end/e'),
            ('end/s',           'edge/vert',        'end/n'),
            ('corner/nw',       'tee/s',            'corner/ne'),
            ('tee/e',           'cross',            'tee/w'),
            ('corner/sw',       'tee/n',            'corner/se'),
            ('end/fancy/e',     'gate',             'end/fancy/w'),
            )

    b = structure_builder()
    for i, row in enumerate(fence_parts):
        for j, part_name in enumerate(row):
            name = basename + '/' + part_name
            b.merge(mk_solid_small(name, image, base=(j, i)))

    return b

def init():
    structures = loader('structures')

    fence = do_fence_parts('fence', structures('fence.png'))

    i = item_builder()
    i.merge(mk_structure_item(fence['fence/edge/horiz'], 'fence', 'Fence'))
    i.merge(mk_structure_item(fence['fence/tee/e'], 'fence_tee', 'Fence Tee'))
    i.merge(mk_structure_item(fence['fence/end/fancy/e'], 'fence_post', 'Fence Post'))
    i.recipe('anvil', {'wood': 5})
