from ..core.builder import *
from ..core import util


def gen_default_anim(b, maps):
    return maps.animations['pony/stand-0']

def gen_editor_anim(b, maps):
    return maps.animations['pony/stand-4']

def gen_physics_anim_table(b, maps):
    SPEED_NAMES = ('stand', 'walk', None, 'run')
    table = []
    for speed in SPEED_NAMES:
        if speed is None:
            table.append(None)
            continue

        table.append([maps.animations['pony/%s-%d' % (speed, dir_)]
            for dir_ in range(8)])

    return table

def gen_anim_dir_table(b, maps):
    dct = {}
    for speed in ('stand', 'run', 'walk'):
        for dir_ in range(8):
            anim_id = maps.animations['pony/%s-%d' % (speed, dir_)]
            dct[anim_id] = dir_
    return dct

def gen_pony_slot_table(b, maps):
    result = []
    for sex in ('f', 'm'):
        parts = {}
        for part in ('base', 'mane', 'tail', 'eyes', 'equip0', 'equip1', 'equip2'):
            # TODO: replace .get() with []
            parts[part] = maps.attach_slots.get('pony/%s/%s' % (sex, part))
        result.append(parts)
    return result

def gen_pony_bases_table(b, maps):
    result = []
    for bits in range(8):
        # This mimics the logic in client/js/graphics/appearance/pony.js
        tribe_idx = bits & 3
        tribe = ('E', 'P', 'U', 'A')[tribe_idx]
        stallion_bit = (bits >> 2) & 1
        sex = ('f', 'm')[stallion_bit]

        # TODO: replace .get() with []
        result.append(maps.attachments_by_slot.get('pony/%s/base' % sex, {}).get(tribe))
    return result

def init():
    mk_extra('default_anim', gen_default_anim)
    mk_extra('editor_anim', gen_editor_anim)
    mk_extra('physics_anim_table', gen_physics_anim_table)
    mk_extra('anim_dir_table', gen_anim_dir_table)
    mk_extra('pony_slot_table', gen_pony_slot_table)
    mk_extra('pony_bases_table', gen_pony_bases_table)
