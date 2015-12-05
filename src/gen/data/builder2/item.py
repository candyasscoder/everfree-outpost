from PIL import Image

from outpost_data.core.builder2.base import *
from outpost_data.core.builder2.structure import StructureBuilder, StructurePrototype
from outpost_data.core.consts import *
from outpost_data.core.item import ItemDef


class ItemPrototype(PrototypeBase):
    KIND = 'item'
    FIELDS = ('display_name', 'icon')

    def instantiate(self):
        self.name = self.require('name') or '_%x' % id(self)
        display_name = self.require('display_name', default=self.name)
        icon = raw_image(self.require('icon'))
        return ItemDef(self.name, display_name, icon)

def make_structure_icon(orig):
    w, h = orig.size
    side = max(w, h)
    img = Image.new('RGBA', (side, side))
    img.paste(orig, ((side - w) // 2, (side - h) // 2))
    return img.resize((TILE_SIZE, TILE_SIZE), resample=Image.ANTIALIAS)

class ItemBuilder(BuilderBase):
    PROTO_CLASS = ItemPrototype

    display_name = dict_modifier('display_name')

    @dict_setter
    def icon(self, icon):
        if icon.px_size != (TILE_SIZE, TILE_SIZE):
            icon = icon.scale((1, 1), unit=TILE_SIZE)
        self.icon = icon

    def from_structure(self, s, name=None, extract_offset=None, **kwargs):
        if isinstance(s, StructurePrototype):
            s = [s]
        elif isinstance(s, StructureBuilder):
            s = list(s._dct.values())
        if len(s) > 1:
            assert name is None, "can't provide a name when generating multiple items"

        child = self.child()

        for s in s:
            if extract_offset is None:
                icon = s.get_image().modify(make_structure_icon, unit=TILE_SIZE, size=1)
            else:
                icon = s.get_image().extract(extract_offset, TILE_SIZE, unit=1)
                icon.set_unit(TILE_SIZE)

            child.new(name or s.name).icon(icon)

        child._apply_kwargs(kwargs)
        return child
