from outpost_data.core.consts import *
from outpost_data.core.builder2 import *
from outpost_data.core.image2 import load
from outpost_data.core import structure
from outpost_data.outpost.lib import models

FENCE_PART_TABLE = (
        ('end/w',           'edge/horiz',       'end/e'),
        ('end/s',           'edge/vert',        'end/n'),
        ('corner/nw',       'tee/s',            'corner/ne'),
        ('tee/e',           'cross',            'tee/w'),
        ('corner/sw',       'tee/n',            'corner/se'),
        ('end/fancy/e',     'gate',             'end/fancy/w'),
        )
FENCE_PARTS = {v: (x, y) for y, vs in enumerate(FENCE_PART_TABLE) for x, v in enumerate(vs)}

FENCE_ITEMS = (
        ('fence', 'Fence', 'edge/horiz'),
        ('fence_tee', 'Fence Tee', 'tee/e'),
        ('fence_post', 'Fence Post', 'end/fancy/e'),
        )

def init():
    sheet = load('structures/fence.png', unit=TILE_SIZE)
    parts = sheet.chop(FENCE_PARTS)

    s = STRUCTURE.prefixed('fence') \
            .model(models.front(1, 1, 1)) \
            .shape(structure.solid(1, 1, 1)) \
            .layer(1)
    for k in FENCE_PARTS.keys():
        s.new(k).image(parts[k])

    r = RECIPE.prefixed('fence') \
            .station('anvil') \
            .input('wood', 5)
    for (name, display_name, struct_name) in FENCE_ITEMS:
        i = ITEM.from_structure(s[struct_name], name=name).display_name(display_name)
        r.from_item(i)
