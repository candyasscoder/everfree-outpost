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

def init():
    mk_extra('default_anim', gen_default_anim)
    mk_extra('editor_anim', gen_editor_anim)
    mk_extra('physics_anim_table', gen_physics_anim_table)
    mk_extra('anim_dir_table', gen_anim_dir_table)
