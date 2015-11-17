from PIL import Image, ImageChops

from ..core.builder import *
from ..core.images import loader
from ..core.animation import AnimGroupDef
from ..core import util

from .lib import pony_sprites


LAYER_NAMES = ('base', 'horn', 'frontwing', 'backwing')

def mk_layer_sheets(ms):
    get_img = loader('sprites/base/%s' % ms)
    sheets = {}

    for l in LAYER_NAMES:
        parts = {}
        for i in range(5):
            j = pony_sprites.INV_DIRS[i]
            img = get_img('%s-%d-%s.png' % (ms, i, l))
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

def mk_base_sheets(ms):
    layer_sheets = mk_layer_sheets(ms)
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

    group = pony_sprites.get_anim_group()

    offsets = pony_sprites.get_hat_offsets()

    slots = {}

    for sex, ms in (('f', 'mare'), ('m', 'stallion')):
        # Define slots
        cur_slots = {}
        for part in ('base', 'mane', 'tail', 'eyes', 'equip0', 'equip1', 'equip2'):
            cur_slots[part] = mk_attach_slot('pony/%s/%s' % (sex, part), group)
        slots[sex] = cur_slots

        # Add base variants
        sheets = mk_base_sheets(ms)
        for k in BASES.keys():
            base = mk_sprite('pony/%s/base/%s' % (sex, k), group, (96, 96), sheets[k])
            cur_slots['base'].add_variant(k, base)

        # Add eyes, mane, tail variants
        eyes = pony_sprites.mk_hat_sheets(sprites('parts/%s/eyes1.png' % ms),
                group, offsets[ms], 110)
        sprite = mk_sprite('pony/%s/eyes/0' % sex, group, (96, 96), eyes)
        cur_slots['eyes'].add_variant('0', sprite)

        for i in range(3):
            for part in ('mane', 'tail'):
                sprite = mk_sprite('pony/%s/%s/%d' % (sex, part, i), group, (96, 96),
                        mk_hair_sheets(sprites('parts/%s/%s%d.png' % (ms, part, i + 1)), 120))
                cur_slots[part].add_variant(str(i), sprite)

        # Add `none` variant for equip slots
        for i in range(3):
            cur_slots['equip%d' % i].add_variant('none', None)


    # Add hat variants to equip0 slot
    hat_base_f = sprites('equipment/witch-hat-f.png')
    hat_f = mk_sprite('pony/f/equip0/0', group, (96, 96),
            pony_sprites.mk_hat_sheets(hat_base_f, group, offsets['mare'], 130))
    slots['f']['equip0'].add_variant('0', hat_f)

    hat_base_m = sprites('equipment/witch-hat-m.png')
    hat_m = mk_sprite('pony/m/equip0/0', group, (96, 96),
            pony_sprites.mk_hat_sheets(hat_base_m, group, offsets['stallion'], 130))
    slots['m']['equip0'].add_variant('0', hat_m)


    hat_base_f = sprites('equipment/party-hat-f.png')
    hat_f = mk_sprite('pony/f/equip0/party', group, (96, 96),
            pony_sprites.mk_hat_sheets(hat_base_f, group, offsets['mare'], 130))
    slots['f']['equip0'].add_variant('party', hat_f)

    hat_base_m = sprites('equipment/party-hat-m.png')
    hat_m = mk_sprite('pony/m/equip0/party', group, (96, 96),
            pony_sprites.mk_hat_sheets(hat_base_m, group, offsets['stallion'], 130))
    slots['m']['equip0'].add_variant('party', hat_m)
