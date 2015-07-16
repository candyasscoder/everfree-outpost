from PIL import Image, ImageChops

from ..core.builder import *
from ..core.images import loader
from ..core.animation import AnimGroupDef
from ..core import util

from .lib import pony_sprites


INV_DIRS = [None] * 5
for i, info in enumerate(pony_sprites.DIRS):
    if 'mirror' not in info:
        INV_DIRS[info['idx']] = i


LAYER_NAMES = ('base', 'horn', 'frontwing', 'backwing')

def sheets_from_parts(group, parts, size):
    w,h = size
    sheets = [Image.new('RGBA', (sw * w, sh * h)) for (sw, sh) in group.sheet_sizes]
    for name, img in parts.items():
        anim = group.anims.get(name)
        if anim is None:
            util.err('no animation %r in group %r' % (name, group.name))
            continue

        cur_sheet = sheets[anim.sheet]
        x,y = anim.offset
        cur_sheet.paste(img, (x * w, y * h))
    return sheets

def mk_layer_sheets():
    mare = loader('sprites/base/mare')
    sheets = {}

    for l in LAYER_NAMES:
        parts = {}
        for i in range(5):
            j = INV_DIRS[i]
            img = mare('mare-%d-%s.png' % (i, l))
            take = lambda x, y, w: img.crop((x * 96, y * 96, (x + w) * 96, (y + 1) * 96))

            parts['stand-%d' % j] = take(0, 0, 1)
            parts['walk-%d' % j] = take(0, 1, 6)
            parts['run-%d' % j] = take(0, 3, 6)

        sheets[l] = sheets_from_parts(pony_sprites.get_anim_group(), parts, (96, 96))

    return sheets


BASES = {
        'E': ('base',),
        'P': ('backwing', 'base', 'frontwing'),
        'U': ('base', 'horn'),
        'A': ('backwing', 'base', 'horn', 'frontwing'),
        }

LAYER_DEPTHS = {
        'base': 100,
        'horn': 150,
        'frontwing': 150,
        'backwing': 50,
        }

def mk_base_sheets():
    layer_sheets = mk_layer_sheets()
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
        base_sheets[name] = sheets
    return base_sheets


def init():
    sheets = mk_base_sheets()
    group = pony_sprites.get_anim_group()

    for k in BASES.keys():
        mk_sprite('pony/f/%s' % k, group, (96, 96), sheets[k])
