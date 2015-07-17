from PIL import Image

from .consts import *
from .util import pack_boxes, warn

class AnimationDef(object):
    def __init__(self, group, name, sheet, offset, length, framerate, mirror=False):
        self.group = group
        self.name = name
        self.sheet = sheet
        self.offset = offset
        self.length = length
        self.framerate = framerate
        self.mirror = mirror

        self.id = None

class AnimGroupDef(object):
    def __init__(self, name):
        self.name = name
        self.anim_info = {}
        self.anim_mirrors = {}

        self.masks = {}
        self.boxes = {}

        self.anims = None
        self.sheet_sizes = None

        self.id = None

    def add_anim(self, name, length, framerate, mirror=False):
        assert name not in self.anim_info
        self.anim_info[name] = {
                'length': length,
                'framerate': framerate,
                'mirror': mirror,
                }

    def add_anim_mirror(self, name, orig_name):
        assert name not in self.anim_mirrors
        self.anim_mirrors[name] = orig_name

    def finish(self):
        anim_names = sorted(self.anim_info.keys())
        size = 2048 // 128
        boxes = [(self.anim_info[n]['length'], 1) for n in anim_names]
        num_sheets, offsets = pack_boxes((size, size), boxes)

        anims = {}
        sheet_w = [0] * num_sheets
        sheet_h = [0] * num_sheets

        for name, (sheet_idx, offset) in zip(anim_names, offsets):
            info = self.anim_info[name]
            full_name = '%s/%s' % (self.name, name)
            anims[name] = AnimationDef(self, full_name, sheet_idx, offset,
                    info['length'], info['framerate'], info['mirror'])
            sheet_w[sheet_idx] = max(sheet_w[sheet_idx], offset[0] + info['length'])
            sheet_h[sheet_idx] = max(sheet_h[sheet_idx], offset[1] + 1)

        for name, orig_name in self.anim_mirrors.items():
            assert name not in anims
            orig = anims[orig_name]
            full_name = '%s/%s' % (self.name, name)
            anims[name] = AnimationDef(self, full_name, orig.sheet, orig.offset, orig.length,
                    orig.framerate, mirror=not orig.mirror)

        self.anims = anims
        self.sheet_sizes = list(zip(sheet_w, sheet_h))

class SpriteDef(object):
    def __init__(self, name, group, size, images):
        self.name = name
        self.group = group
        self.size = size
        self.images = images

        assert len(self.images) == len(self.group.sheet_sizes)
        sw, sh = self.size
        for img, (gw, gh) in zip(self.images, self.group.sheet_sizes):
            iw, ih = img.size
            assert sw * gw == iw
            assert sh * gh == ih


# JSON output

def build_client_json(animations):
    def convert(a):
        return {
                'sheet': a.sheet,
                'offset': a.offset,
                'length': a.length,
                'framerate': a.framerate,
                'mirror': a.mirror,
                }
    return list(convert(a) for a in animations)

def build_server_json(animations):
    def convert(a):
        return {
                'name': a.name,
                'length': a.length,
                'framerate': a.framerate,
                }
    return list(convert(a) for a in animations)

