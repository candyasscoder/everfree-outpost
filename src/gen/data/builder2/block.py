from PIL import Image

from outpost_data.core.builder2.base import *
from outpost_data.core.consts import *
from outpost_data.core.block import BlockDef


class BlockPrototype(PrototypeBase):
    KIND = 'block'
    FIELDS = ('shape', 'top', 'bottom', 'front', 'back')

    def instantiate(self):
        name = self.require('name') or '_%x' % id(self)
        shape = self.require('shape') or 'solid'
        tiles = {}
        for side in BLOCK_SIDES:
            x = getattr(self, side)
            if x is not None:
                tiles[side] = raw_image(x)

        return BlockDef(name, shape, tiles)

class BlockBuilder(BuilderBase):
    PROTO_CLASS = BlockPrototype

    shape = dict_modifier('shape')
    top = dict_modifier('top')
    bottom = dict_modifier('bottom')
    front = dict_modifier('front')
    back = dict_modifier('back')
