from ..structure import empty as empty_shape, floor as floor_shape, solid as solid_shape
from ..consts import *


class InterpError(Exception):
    pass


def field_meta(name, bases, attrs):
    for k in attrs:
        attrs[k] = staticmethod(attrs[k])

    attrs['__new__'] = lambda cls: cls

class FieldBase(object):
    __metaclass__ = field_meta
    IS_INIT_FIELD = False

    def eval(interp, val):
        return val

    def apply(obj, name, val):
        getattr(obj, name)(val)

class InitFieldBase(FieldBase):
    IS_INIT_FIELD = True

    def apply(obj, name, val):
        raise InterpError('this field must be placed first in the section')

class NameField(FieldBase):
    def parse(parser):
        return parser.take_word()

class StringField(FieldBase):
    def parse(parser):
        return parser.take_quoted()

class IntField(FieldBase):
    def parse(parser):
        return parser.take_int()

class ImageField(FieldBase):
    def parse(parser):
        return parser.parse_filename()

    def eval(interp, val):
        return interp.load_image(val)

class ItemCountField(FieldBase):
    def parse(parser):
        count = parser.take_int()
        item = parser.take_word()
        return (count, item)

    def apply(obj, name, val):
        if isinstance(val, tuple):
            count, item = val
            getattr(obj, name)(item, count)
        else:
            getattr(obj, name)(val)

class ShapeField(FieldBase):
    def parse(parser):
        t = parser.peek()
        kind = parser.take_word()
        if kind not in ('solid', 'floor', 'empty'):
            parser.error('"solid", "floor", or "empty"', t)

        parser.take_punct('(')
        x = parser.take_int()
        parser.take_punct(',')
        y = parser.take_int()
        parser.take_punct(',')
        z = parser.take_int()
        parser.take_punct(')')

        if kind == 'solid':
            return solid_shape(x, y, z)
        elif kind == 'floor':
            return floor_shape(x, y, z)
        elif kind == 'empty':
            return empty_shape(x, y, z)
        else:
            assert False, 'should have bailed out due to parser.error above'

class MultiNameField(InitFieldBase):
    def parse(parser):
        parser.error('backticked Python expression')

    def init(interp, section, name, val):
        key = section.ty
        if key.startswith('multi_'):
            key = key[len('multi_'):]
        builder = interp.builders[key]
        return builder.prefixed(section.name).new(val)

class FromObjectField(InitFieldBase):
    def parse(parser):
        return parser.take_word()

    def init(interp, section, name, val):
        assert name.startswith('from_')
        template_builder = interp.builders[name[len('from_'):]]
        assert isinstance(val, str)
        template_obj = template_builder[val]

        builder = interp.builders[section.ty]
        from_obj = getattr(builder, name)
        return from_obj(template_obj, name=section.name)

def _build_field_map():
    fm = {}
    fm['structure'] = dict(
            multi_names = MultiNameField,
            image = ImageField,
            depthmap = ImageField,
            shape = ShapeField,
            layer = IntField,
            )
    fm['item'] = dict(
            from_structure = FromObjectField,
            icon = ImageField,
            display_name = StringField,
            )
    fm['recipe'] = dict(
            from_item = FromObjectField,
            display_name = StringField,
            station = NameField,
            input = ItemCountField,
            output = ItemCountField
            )

    return fm

FIELD_MAP = _build_field_map()
