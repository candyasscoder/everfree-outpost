from outpost_data.core.builder2.base import *
from outpost_data.core.builder2.item import ItemBuilder, ItemPrototype
from outpost_data.core.consts import *
from outpost_data.core.loot_table import LootTableDef


class LootTablePrototype(PrototypeBase):
    KIND = 'loot_table'
    FIELDS = ('table', 'extension')

    def __init__(self):
        super(LootTablePrototype, self).__init__()
        self.extension = False

    def clone(self):
        obj = super(LootTablePrototype, self).clone()
        obj.table = self.table.clone() if self.table is not None else None
        return obj

    def instantiate(self):
        self.name = self.require('name') or '_%x' % id(self)
        table = self.require('table')
        ext = self.extension
        return LootTableDef(self.name, table, ext)

class LootTableBuilder(BuilderBase):
    PROTO_CLASS = LootTablePrototype

    def _add(self, name, val):
        # NB: duplicated from BuilderBase
        # Use unique names internally, to allow for `_ext` tables named the
        # same as the originals.
        self._dct['%s_%s' % (name, id(val))] = val
        if self._parent is not None:
            self._parent._add(self._prefix + name, val)

    table = dict_modifier('table')
    set_extension = dict_modifier('extension')

    def extension(self):
        def f(x, arg):
            x.extension = True
        self._modify(f, ())
