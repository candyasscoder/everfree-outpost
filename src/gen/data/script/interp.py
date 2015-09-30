from .. import builder2, image2, util
from ..consts import *

from .field import FIELD_MAP, InterpError
from .parse import Backticked, Value

class Interpreter(object):
    def __init__(self, builders, ctx, load_image=None):
        self.builders = builders
        self.ctx = ctx
        self.load_image = load_image or image2.loader()

    def run_script(self, script):
        for section in script:
            self.run_section(section)

    def run_section(self, sect):
        field_map = FIELD_MAP[sect.ty]

        try:
            has_init_field = len(sect.fields) > 0 and field_map[sect.fields[0].name].IS_INIT_FIELD
            if has_init_field:
                obj = self.run_init_field(sect, sect.fields[0])
            else:
                obj = self.builders[sect.ty].new(sect.name)
        except (InterpError, AssertionError) as e:
            util.err('%s %r: error during initialization: %s' %
                    (sect.ty, sect.name, e))
            return

        for f in (sect.fields[1:] if has_init_field else sect.fields):
            try:
                self.run_field(obj, f, field_map[f.name])
            except InterpError as e:
                util.err('%s %r: field %r: %s' % (sect.ty, sect.name, f.name, e))
                continue

    def eval_field(self, field, info):
        if isinstance(field.value, Backticked):
            try:
                return eval(field.value.src, self.ctx, {
                    'interp': self,
                    'load': self.load_image,
                    'builders': self.builders,
                    })
            except Exception as e:
                raise InterpError(e)
        else:
            return info.eval(self, field.value.value)

    def run_init_field(self, sect, field):
        info = FIELD_MAP[sect.ty][field.name]
        val = self.eval_field(field, info)
        return info.init(self, sect, field.name, val)

    def run_field(self, obj, field, info):
        val = self.eval_field(field, info)
        info.apply(obj, field.name, val)
