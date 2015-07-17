from .consts import *
from . import util


class AttachmentDef(object):
    def __init__(self, slot, name, sprite):
        self.slot = slot
        self.name = name
        self.sprite = sprite

        self.id = None

class AttachSlotDef(object):
    def __init__(self, name, anim_group):
        self.name = name
        self.anim_group = anim_group
        self.variants = []

        self.id = None

    def add_variant(self, name, sprite):
        self.variants.append(AttachmentDef(self, name, sprite))


# JSON output

def build_client_json(slots):
    def convert(s):
        return {
                'name': '%s/%s' % (s.anim_group.name, s.name),
                'variants': [{
                    'name': a.name,
                    'sprite_file': a.sprite.name.replace('/', '_'),
                    } for a in s.variants],
                }
    return list(convert(s) for s in slots)

def build_server_json(slots):
    def convert(s):
        return {
                'name': '%s/%s' % (s.anim_group.name, s.name),
                'variant_names': list(a.name for a in s.variants),
                }
    return list(convert(s) for s in slots)

