from PIL import Image, ImageChops

from ..core.builder import *
from ..core.images import loader
from ..core.animation import AnimGroupDef
from ..core import util

from .lib import pony_sprites


LAYER_NAMES = ('base', 'horn', 'frontwing', 'backwing')

def mk_layer_sheets():
    mare = loader('sprites/base/mare')
    sheets = {}

    for l in LAYER_NAMES:
        parts = {}
        for i in range(5):
            j = pony_sprites.INV_DIRS[i]
            img = mare('mare-%d-%s.png' % (i, l))
            take = lambda x, y, w: img.crop((x * 96, y * 96, (x + w) * 96, (y + 1) * 96))

            parts['stand-%d' % j] = take(0, 0, 1)
            parts['walk-%d' % j] = take(0, 1, 6)
            parts['run-%d' % j] = take(0, 3, 6)

        sheets[l] = pony_sprites.sheets_from_parts(pony_sprites.get_anim_group(), parts, (96, 96))

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


def mk_hair_sheets(img, depth):
    take = lambda x, y, w: img.crop((x * 96, y * 96, (x + w) * 96, (y + 1) * 96))

    parts = {}
    for i in range(5):
        j = pony_sprites.INV_DIRS[i]
        parts['stand-%d' % j] = take(i, 0, 1)
        parts['walk-%d' % j] = take(i * 6, 1, 6)
        parts['run-%d' % j] = take(i * 6, 3, 6)

    sheets = pony_sprites.sheets_from_parts(pony_sprites.get_anim_group(), parts, (96, 96))
    pony_sprites.set_alpha(sheets, depth)
    return sheets


def init():
    sprites = loader('sprites')

    sheets = mk_base_sheets()
    group = pony_sprites.get_anim_group()

    for k in BASES.keys():
        mk_sprite('pony/base/f/%s' % k, group, (96, 96), sheets[k])

    eyes = mk_sprite('pony/eyes/f/0', group, (96, 96),
            mk_hair_sheets(sprites('type1blue.png'), 110))
    mane = mk_sprite('pony/mane/f/0', group, (96, 96),
            mk_hair_sheets(sprites('maremane1.png'), 120))
    tail = mk_sprite('pony/tail/f/0', group, (96, 96),
            mk_hair_sheets(sprites('maretail1.png'), 120))

    mk_attach_slot('eyes', group).add_variant('0', eyes)
    mk_attach_slot('mane', group).add_variant('0', mane)
    mk_attach_slot('tail', group).add_variant('0', tail)

    hat_base = sprites('equipment/witch-hat.png')
    offsets = pony_sprites.get_hat_offsets()['mare']
    hat = mk_sprite('pony/equip0/f/0', group, (96, 96),
            pony_sprites.mk_hat_sheets(hat_base, group, offsets, 130))
    mk_attach_slot('hat', group).add_variant('0', hat)
