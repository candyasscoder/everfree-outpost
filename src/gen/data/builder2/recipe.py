from outpost_data.core.builder2.base import *
from outpost_data.core.builder2.item import ItemBuilder, ItemPrototype
from outpost_data.core.consts import *
from outpost_data.core.recipe import RecipeDef


class RecipePrototype(PrototypeBase):
    KIND = 'recipe'
    FIELDS = ('display_name', 'station', 'inputs', 'outputs')
    def __init__(self):
        super(RecipePrototype, self).__init__()
        self.inputs = {}
        self.outputs = {}

    def clone(self):
        obj = super(RecipePrototype, self).clone()
        obj.inputs = self.inputs.copy()
        obj.outputs = self.outputs.copy()
        return obj

    def instantiate(self):
        self.name = self.require('name') or '_%x' % id(self)
        display_name = self.require('display_name', default=self.name)
        station = self.require('station', default='anvil')
        return RecipeDef(self.name, display_name, station, self.inputs, self.outputs)

class RecipeBuilder(BuilderBase):
    PROTO_CLASS = RecipePrototype

    display_name = dict_modifier('display_name')
    station = dict_modifier('station')
    # `inputs` and `outputs` are already dicts, so there's no way for `_dict_modifier`
    # to distinguish the "set all" and "set named" cases.
    inputs = modifier('inputs')
    outputs = modifier('outputs')

    def input(self, *args):
        """Add a single item to the recipe inputs.  Call either as `x.input(item, count)`
        or as `x.input({'recipe_name': (item, count), ...})` for multiple updates."""
        if len(args) == 1:
            args, = args

        def f(x, item_count):
            item, count = item_count
            x.inputs[item] = count
        return self._dict_modify(f, args)

    def output(self, *args):
        """Add a single item to the recipe outputs.  Call either as `x.output(item, count)`
        or as `x.output({'recipe_name': (item, count), ...})` for multiple updates."""
        if len(args) == 1:
            args, = args

        def f(x, item_count):
            item, count = item_count
            x.outputs[item] = count
        return self._dict_modify(f, args)
        self._modify

    def from_item(self, i, name=None, **kwargs):
        if isinstance(i, ItemPrototype):
            i = [i]
        elif isinstance(i, ItemBuilder):
            i = list(i._dct.values())
        if len(i) > 1:
            assert name is None, "can't provide a name when generating multiple recipes"

        child = self.child()

        for i in i:
            child.new(name or i.name) \
                    .display_name(i.display_name) \
                    .output(i.name, 1)

        child._apply_kwargs(kwargs)
        return child
