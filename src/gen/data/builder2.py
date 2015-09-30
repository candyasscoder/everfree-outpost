from PIL import Image

from .consts import *
from . import util

from .structure import solid, StructureDef, StaticAnimDef
from .item import ItemDef
from .recipe import RecipeDef


DEFAULT_IMAGE = Image.new('RGBA', (TILE_SIZE, TILE_SIZE))
DEFAULT_SHAPE = solid(1, 1, 1)

def raw_image(img):
    if img is None:
        return DEFAULT_IMAGE
    else:
        return img.raw()


class PrototypeBase(object):
    def __init__(self):
        self.name = None
        for k in type(self).FIELDS:
            setattr(self, k, None)

    def clone(self):
        new_inst = type(self)()
        for k in type(self).FIELDS:
            setattr(new_inst, k, getattr(self, k))
        return new_inst

    def require(self, field, default=None, reason=None):
        val = getattr(self, field)
        if val is None:
            if reason:
                util.err('%s %r: field %r must be set because %r is set' %
                        (self.KIND, self.name, field, reason))
            else:
                util.err('%s %r: field %r must be set' %
                        (self.KIND, self.name, field))
            return default
        else:
            return val

    def require_unset(self, field, reason):
        if getattr(self, field) is not None:
            util.err('%s %r: field %r must not be set because %r is set' %
                    (self.KIND, self.name, field, reason))

    def require_one(self, field1, field2):
        """Check that exactly one of the two fields is set.  Returns True if
        the first is set, otherwise False.  Reports an error and defaults to
        True if both or neither are set."""
        val1 = getattr(self, field1)
        val2 = getattr(self, field2)
        if val1 is not None and val2 is not None:
            util.err('%s %r: field %r and field %r must not both be set' %
                    (self.KIND, self.name, field1, field2))
            return True
        elif val1 is not None:
            return True
        elif val2 is not None:
            return False
        else:
            util.err('%s %r: either field %r or field %r must be set' %
                    (self.KIND, self.name, field1, field2))
            return True

    def check_group(self, fields, optional=()):
        """Get all fields in a group.  If any field in the group is set, then
        all required fields must be set or an error will be reported."""
        result = [None] * (len(fields) + len(optional))
        first_set = None
        all_set = True

        i = 0

        for f in fields:
            val = getattr(self, f)
            if val is not None:
                first_set = first_set or f
                result[i] = val
            else:
                all_set = False
            i += 1

        for f in optional:
            val = getattr(self, f)
            if val is not None:
                first_set = first_set or f
                result[i] = val
            i += 1

        if first_set is not None and not all_set:
            for f in fields:
                if getattr(self, f) is None:
                    util.err('%s %r: field %r must be set because %r is set' %
                            (self.KIND, self.name, f, first_set))

        return result


def setter(f):
    return lambda x, v: setattr(x, f, v)

def dict_modifier(f):
    s = setter(f)
    return lambda self, arg: self._dict_modify(s, arg)

def modifier(f):
    s = setter(f)
    return lambda self, arg: self._modify(s, arg)


class BuilderBase(object):
    def __init__(self, parent=None, prefix=None):
        self._dct = {}
        self._parent = parent
        self._proto = parent._proto.clone() if parent is not None else (self.PROTO_CLASS)()

        self._prefix = prefix + '/' if prefix is not None else ''
        self._full_prefix = (parent._full_prefix if parent is not None else '') + self._prefix

    def _add(self, name, val):
        assert name not in self._dct, \
                'duplicate %s with name %r' % (self.PROTO_CLASS.KIND, name)
        self._dct[name] = val
        if self._parent is not None:
            self._parent._add(self._prefix + name, val)

    def _modify(self, f, arg):
        f(self._proto, arg)
        for x in self._dct.values():
            f(x, arg)
        return self

    def _dict_modify(self, f, arg):
        if isinstance(arg, dict):
            for k, v in arg.items():
                f(self._dct[k], v)
        else:
            f(self._proto, arg)
            for x in self._dct.values():
                f(x, arg)
        return self

    def _apply_kwargs(self, dct):
        for k, v in dct.items():
            getattr(self, k)(v)

    def __getitem__(self, k):
        return self._dct[k]

    def _create(self, name):
        obj = self._proto.clone()
        obj.name = self._full_prefix + name
        return obj

    def child(self):
        return type(self)(parent=self)

    def prefixed(self, prefix):
        return type(self)(parent=self, prefix=prefix)

    def new(self, name, **kwargs):
        result = self.child()

        if isinstance(name, str):
            result._add(name, self._create(name))
        else:
            for name in name:
                result._add(name, self._create(name))

        result._apply_kwargs(kwargs)
        return result

    def from_clone(self, p, **kwargs):
        if isinstance(p, BuilderBase):
            p = p._dct

        result = self.child()
        if isinstance(p, PrototypeBase):
            result._add(p.name, p)
        else:
            for p in p:
                result._add(p.name, p)

        result._apply_kwargs(kwargs)
        return result

    def unwrap(self):
        assert len(self._dct) == 1, \
                'unwrap: expected a single element, but got %d (%s)' % \
                (len(self._dct), sorted(self._dct.keys()))
        for x in self._dct.values():
            return x



class StructurePrototype(PrototypeBase):
    KIND = 'structure'
    FIELDS = (
            'image', 'depthmap', 'shape', 'layer',
            'light_offset', 'light_color', 'light_radius',
            'anim_frames', 'anim_framerate', 'anim_oneshot',
            )

    def instantiate(self):
        self.name = self.require('name') or '_%x' % id(self)

        if self.require_one('image', 'anim_frames'):
            img = raw_image(self.image)
        else:
            frames = self.anim_frames
            rate = self.require('anim_framerate', default=1, reason='anim_frames')
            oneshot = self.anim_oneshot or False

            img = StaticAnimDef(self.anim_frames, self.anim_framerate, self.anim_oneshot)

        depthmap = raw_image(self.require('depthmap'))
        shape = self.require('shape', DEFAULT_SHAPE)
        layer = self.require('layer', 0)

        s = StructureDef(self.name, img, depthmap, shape, layer)

        pos, color, radius = self.check_group(
                ('light_offset', 'light_color', 'light_radius'))
        if pos is not None:
            s.set_light(pos or (0, 0, 0), color or (0, 0, 0), radius or 1)

        return s

    def get_image(self):
        if self.image is not None:
            return self.image
        elif self.anim_frames is not None and len(self.anim_frames) > 0:
            return self.anim_frames[0]
        else:
            return DEFAULT_IMAGE

class StructureBuilder(BuilderBase):
    PROTO_CLASS = StructurePrototype

    image = dict_modifier('image')
    depthmap = dict_modifier('depthmap')
    shape = dict_modifier('shape')
    layer = dict_modifier('layer')

    light_offset = dict_modifier('light_offset')
    light_color = dict_modifier('light_color')
    light_radius = dict_modifier('light_radius')

    anim_frames = dict_modifier('anim_frames')
    anim_framerate = dict_modifier('anim_framerate')
    anim_oneshot = dict_modifier('anim_oneshot')

    def light(offset, color, radius):
        def f(x):
            x.light_offset = offset
            x.light_color = color
            x.light_radius = radius
        return self._modify(f)

    def anim(frames, framerate, oneshot=False):
        def f(x):
            x.anim_frames = frames
            x.anim_framerate = framerate
            x.anim_oneshot = oneshot
        return self._modify(f)


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
    icon = dict_modifier('icon')

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
                icon = s.get_image().modify(make_structure_icon, unit=TILE_SIZE)
            else:
                icon = s.get_image().extract(extract_offset, TILE_SIZE, unit=1)
                icon.set_unit(TILE_SIZE)

            child.new(name or s.name).icon(icon)

        child._apply_kwargs(kwargs)
        return child


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
        else:
            assert name is None, "can't provide a name when generating multiple recipes"
            if isinstance(i, ItemBuilder):
                i = list(i._dct.values())

        child = self.child()

        for i in i:
            child.new(name or i.name) \
                    .display_name(i.display_name) \
                    .output(i.name, 1)

        child._apply_kwargs(kwargs)
        return child
