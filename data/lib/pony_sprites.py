import os

from ...core.builder import *
from ...core.images import load
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

INV_DIRS = [None] * 5
for i, info in enumerate(DIRS):
    if 'mirror' not in info:
        INV_DIRS[info['idx']] = i


MOTIONS = [
        {'name': 'stand', 'row': 0, 'base_col': 0, 'len': 1, 'fps': 1},
        {'name': 'walk', 'row': 1, 'base_col': 0, 'len': 6, 'fps': 8},
        {'name': 'run', 'row': 3, 'base_col': 0, 'len': 6, 'fps': 12},
        ]

SPRITE_SIZE = (96, 96)

@cached
def get_anim_group():
    g = mk_anim_group('pony')

    for m in MOTIONS:
        for i, d in enumerate(DIRS):
            mirror = d.get('mirror')
            if mirror is None:
                g.add_anim('%s-%d' % (m['name'], i), m['len'], m['fps'])
            else:
                g.add_anim_mirror('%s-%d' % (m['name'], i),
                        '%s-%d' % (m['name'], mirror))
    g.finish()
    return g.unwrap()

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

def find_box_containing(alpha, pos):
    if alpha.getpixel(pos) == 0:
        return None

    x, y = pos
    while y > 0 and alpha.getpixel((x, y - 1)) != 0:
        y -= 1
    while x > 0 and alpha.getpixel((x - 1, y)) != 0:
        x -= 1
    return (x, y)

@cached
def get_hat_offsets():
    offsets = {}

    sw, sh = SPRITE_SIZE
    def get_offset(alpha, x, y):
        cx = x * sw + sw // 2
        cy = y * sh + sh // 2
        px_x, px_y = find_box_containing(alpha, (cx, cy))
        return (px_x - x * sw, px_y - y * sh)

    for ms in ('mare', 'stallion'):
        cur_offsets = {}
        for facing in range(5):
            path = os.path.join('sprites/base', ms, '%s-%d-hat-box.png' % (ms, facing))
            image = load(path)
            alpha = image.split()[3]

            for m in MOTIONS:
                row = [get_offset(alpha, m['base_col'] + i, m['row']) for i in range(m['len'])]
                cur_offsets['%s-%d' % (m['name'], facing)] = row
        offsets[ms] = cur_offsets
    return offsets

def set_alpha(sheets, alpha):
    for img in sheets:
        mask = img.split()[3]
        mask = mask.point(lambda x: alpha if x == 255 else 0)
        img.putalpha(mask)

HAT_SIZE = (64, 64)

def mk_hat_sheets(hat_base, group, offsets, depth):
    parts = {}
    sw, sh = SPRITE_SIZE
    hw, hh = HAT_SIZE

    for i in range(5):
        j = INV_DIRS[i]
        hat = hat_base.crop((i * hw, 0, (i + 1) * hw, hh))

        for m in MOTIONS:
            name = m['name']
            name_i = '%s-%d' % (name, i)
            name_j = '%s-%d' % (name, j)

            if name_i not in offsets:
                continue

            frames = len(offsets[name_i])
            part = Image.new('RGBA', (frames * sw, sh))

            for f in range(frames):
                ox, oy = offsets[name_i][f]
                part.paste(hat, (ox + sw * f, oy))

            parts[name_j] = part

    sheets = sheets_from_parts(group, parts, SPRITE_SIZE)
    set_alpha(sheets, depth)
    return sheets
