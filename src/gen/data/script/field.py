import ast

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

    def eval(ctx, val):
        return ctx.const(val)

    def apply(ctx, obj, name, val):
        return obj.attr(name).call(val)

class InitFieldBase(FieldBase):
    IS_INIT_FIELD = True

    def apply(ctx, obj, name, val):
        return ast.Raise(
                ctx.var('RuntimeError').call('this field must be placed first in the section'),
                None)

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

    def eval(ctx, val):
        return ctx.var('load').call(ctx.const(val))

class ItemCountField(FieldBase):
    def parse(parser):
        count = parser.take_int()
        item = parser.take_word()
        return (item, count)

    # NB: input()/output() methods work not only as `input(item, count)` but
    # also as `input((item, count))` (taking a tuple).

class ShapeField(FieldBase):
    def parse(parser):
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
        return (kind, (x, y, z))

    def eval(ctx, val):
        kind, (x, y, z) = val
        return ctx.var('structure').attr(kind) \
                .call(ctx.const(x), ctx.const(y), ctx.const(z))

class MultiNameField(InitFieldBase):
    def parse(parser):
        parser.error('backticked Python expression')

    def init(ctx, section, field_name, val_ast):
        return ctx.var('INSTANCES').index(section.ty) \
                .attr('prefixed').call(section.name) \
                .attr('new').call(val_ast)

class FromObjectField(InitFieldBase):
    def parse(parser):
        return parser.take_word()

    def init(ctx, section, field_name, val_ast):
        assert field_name.startswith('from_')
        template_ty = field_name[len('from_'):]
        template_obj = ctx.var('INSTANCES').index(template_ty).index(val_ast)

        return ctx.var('INSTANCES').index(section.ty) \
                .attr(field_name).call(template_obj, name=section.name)

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
