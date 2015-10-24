import ast

from outpost_data.core.script.defs.field import FIELD_MAP
from outpost_data.core.script.defs.parse import Section, PythonBlock
from outpost_data.core.script.defs.parse import Backticked, Value
from outpost_data.core.script.util import *

class Compiler(object):
    def __init__(self, filename):
        self.filename = filename

    def var(self, name, ctx=None):
        ctx = ctx or ast.Load()
        return ExprBuilder(ast.Name(name, ctx))

    def const(self, val):
        return ExprBuilder(val_to_expr(val))


    def assign(self, lhs, rhs):
        lhs = expr(lhs)
        lhs.ctx = ast.Store()
        return ast.Assign([lhs], expr(rhs))

    def import_from(self, mod, names):
        if isinstance(names, dict):
            return ast.ImportFrom(mod, [ast.alias(k, v) for k,v in names.items()], 0)
        else:
            return ast.ImportFrom(mod, [ast.alias(x, None) for x in names], 0)


    def gen_preamble(self):
        return [
                self.import_from('outpost_data.core',
                    ('structure', 'item', 'recipe')),
                self.import_from('outpost_data.core', ('image2',)),
                self.import_from('outpost_data.core.consts', ('*',)),
                self.import_from('outpost_data.core.builder2', ('INSTANCES',)),
                self.import_from('outpost_data.core.script.defs.context', ('*',)),
                ]

    def gen_init_body(self, script):
        stmts, emit = mk_emit()

        # load = image2.loader()
        emit(self.assign(self.var('load'), self.var('image2').attr('loader').call()))

        for part in script:
            if isinstance(part, Section):
                stmts.extend(self.gen_section(part))
            elif isinstance(part, PythonBlock):
                stmts.extend(self.gen_python_block(part))

        return stmts

    def gen_section(self, sect):
        stmts, emit = mk_emit()

        sect_field_map = FIELD_MAP[sect.ty]

        has_init_field = len(sect.fields) > 0 and sect_field_map[sect.fields[0].name].IS_INIT_FIELD
        if has_init_field:
            field = sect.fields[0]
            info = sect_field_map[field.name]

            val_ast = self.gen_field_value(field.value, info)
            obj_ast = info.init(self, sect, field.name, val_ast)
            emit(self.assign(self.var('obj'), obj_ast), field)
        else:
            # obj = INSTANCES['sect_ty'].new('sect_name')
            obj_ast = self.var('INSTANCES').index(sect.ty) \
                    .attr('new').call(sect.name)
            emit(self.assign(self.var('obj'), obj_ast), sect)

        for field in sect.fields[1:] if has_init_field else sect.fields:
            info = sect_field_map[field.name]
            val_ast = self.gen_field_value(field.value, info)
            emit(info.apply(self, self.var('obj'), field.name, val_ast))

        return stmts

    def gen_python_block(self, block):
        padding = '\n' * block.line
        block_ast = ast.parse(padding + block.code, self.filename, 'exec')
        return block_ast.body

    def gen_field_value(self, val, info):
        if isinstance(val, Backticked):
            # ast.parse(mode='eval') produces a top-level ast.Expression object
            return ast.parse(val.src, filename='<string>', mode='eval').body
        else:
            return info.eval(self, val.value)

    def gen_module(self, script):
        func_args = ast.arguments(args=[])
        func = [ast.FunctionDef(name='init', args=func_args, body=self.gen_init_body(script))]
        m = ast.Module(self.gen_preamble() + func, lineno=0, col_offset=0)
        FuncFix().visit(m)
        ast.fix_missing_locations(m)
        return m

    def compile_module(self, script):
        m = self.gen_module(script)
        return compile(m, self.filename, 'exec')
