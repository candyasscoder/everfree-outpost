import functools

from PIL import Image

from outpost_data.core.consts import *
from outpost_data.core import util


DEFAULT_IMAGE = Image.new('RGBA', (TILE_SIZE, TILE_SIZE))

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

def dict_setter(f):
    @functools.wraps(f)
    def g(self, arg):
        return self._dict_modify(f, arg)
    return g

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
