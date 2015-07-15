from ...core.builder import *
from ...core.images import loader
from ...core.util import cached, err

from PIL import Image, ImageChops


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

INV_DIRS = [None] * 5
for i, info in enumerate(DIRS):
    if 'mirror' not in info:
        INV_DIRS[info['idx']] = i

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


LAYER_NAMES = ('base', 'horn', 'frontwing', 'backwing')

def sheets_from_parts(group, parts, size):
    print(group.name, group.sheet_sizes)
    w,h = size
    sheets = [Image.new('RGBA', (sw * w, sh * h)) for (sw, sh) in group.sheet_sizes]
    for name, img in parts.items():
        anim = group.anims.get(name)
        if anim is None:
            err('no animation %r in group %r' % (name, group.name))
            continue

        cur_sheet = sheets[anim.sheet]
        x,y = anim.offset
        cur_sheet.paste(img, (x * w, y * h))
    return sheets
        

@cached
def get_layer_sheets():
    mare = loader('sprites/base/mare')
    sheets = {}

    for l in LAYER_NAMES:
        parts = {}
        for i in range(5):
            j = INV_DIRS[i]
            img = mare('mare-%d-%s.png' % (i, l))
            take = lambda x, y, w: img.crop((x * 96, y * 96, (x + w) * 96, (y + 1) * 96))

            parts['pony/stand-%d' % j] = take(0, 0, 1)
            parts['pony/walk-%d' % j] = take(0, 1, 6)
            parts['pony/run-%d' % j] = take(0, 3, 6)

        sheets[l] = sheets_from_parts(get_anim_group(), parts, (96, 96))

    return sheets


BASES = {
        'earth_pony': ('base',),
        'pegasus': ('backwing', 'base', 'frontwing'),
        'unicorn': ('base', 'horn'),
        'alicorn': ('backwing', 'base', 'horn', 'frontwing'),
        }

LAYER_DEPTHS = {
        'base': 100,
        'horn': 150,
        'frontwing': 150,
        'backwing': 50,
        }

@cached
def get_base_sheets():
    layer_sheets = get_layer_sheets()
    base_sheets = {}

    for name, layers in BASES.items():
        sheets = []
        for i in range(len(layer_sheets['base'])):
            sheet = Image.new('RGBA', layer_sheets['base'][i].size)
            mask = Image.new('L', sheet.size)

            for l in layers:
                layer_sheet = layer_sheets[l][i]
                sheet.paste(layer_sheet, (0, 0), layer_sheet)

                mask_layer = layer_sheets[l][i].split()[3]
                mask_layer = mask_layer.point(lambda x: LAYER_DEPTHS[l] if x == 255 else 0)
                mask = ImageChops.lighter(mask, mask_layer)

            sheet.putalpha(mask)
            sheets.append(sheet)
            sheets.append(mask)
        base_sheets[name] = sheets
    return base_sheets



