TILE_SIZE = 32
SHEET_SIZE = (32, 32)

SHAPE_ID = {
        'empty': 0,
        'floor': 1,
        'solid': 2,
        'ramp_e': 3,
        'ramp_w': 4,
        'ramp_s': 5,
        'ramp_n': 6,
        'ramp_top': 7,
        }

BLOCK_SIDES = ('front', 'back', 'top', 'bottom')

TERRAIN_PARTS = (
        ('spot/large',      'corner/inner/se',  'corner/inner/sw'),
        ('spot/small',      'corner/inner/ne',  'corner/inner/nw'),
        ('corner/outer/nw', 'edge/n',           'corner/outer/ne'),
        ('edge/w',          'center/v0',        'edge/e'),
        ('corner/outer/sw', 'edge/s',           'corner/outer/se'),
        ('center/v1',       'center/v2',        'center/v3'),
        )
