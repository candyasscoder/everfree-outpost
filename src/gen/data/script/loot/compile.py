import ast
import textwrap

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
                self.import_from('outpost_data.core.loot_table',
                    ('ObjectRef', 'TableRef', 'Weighted', 'Optional')),
                self.import_from('outpost_data.core.loot_table',
                    ('ChooseItem', 'MultiItem', 'ChooseStructure')),
                self.import_from('outpost_data.core.consts', ('*',)),
                self.import_from('outpost_data.core.builder2', ('INSTANCES',)),
                ]

    def gen_init_body(self, script):
        stmts, emit = mk_emit()

        for sect in script:
            stmts.extend(self.gen_section(sect))

        return stmts

    def gen_section(self, sect):
        stmts, emit = mk_emit()

        mode_part = {'choose': 'Choose', 'multi': 'Multi'}[sect.mode]
        obj_kind_part = {'item': 'Item', 'structure': 'Structure'}[sect.obj_kind]
        table_ast = self.var(mode_part + obj_kind_part).call()
        emit(self.assign(self.var('table'), table_ast), sect)

        add_func = {'choose': 'add_variant', 'multi': 'add_part'}[sect.mode]
        for entry in sect.entries:
            if entry.ty == 'object':
                entry_ast = self.var('ObjectRef').call(entry.name, entry.min_count, entry.max_count)
            else:
                entry_ast = self.var('TableRef').call(entry.name)

            if entry.weight is not None:
                entry_ast = self.var('Weighted').call(entry_ast, entry.weight)
            elif entry.chance is not None:
                entry_ast = self.var('Optional').call(entry_ast, entry.chance)

            if sect.mode == 'choose':
                func_ast = self.var('table').attr('add_variant')
            elif sect.mode == 'multi':
                func_ast = self.var('table').attr('add_part')
            else:
                assert False, 'unrecognized section mode: %r' % sect.mode

            emit(func_ast.call(entry_ast), entry)

        builder_ast = self.var('INSTANCES').index('loot_table') \
                .attr('new').call(sect.name) \
                .attr('table').call(self.var('table')) \
                .attr('set_extension').call(sect.ext)
        emit(builder_ast, sect)
        return stmts

    def gen_module(self, script):
        func_args = ast.arguments(args=[])
        func = [ast.FunctionDef(name='init', args=func_args, body=self.gen_init_body(script))]
        m = ast.Module(self.gen_preamble() + func, lineno=0, col_offset=0)
        FuncFix().visit(m)
        ast.fix_missing_locations(m)
        return m

    def compile_module(self, script):
        m = self.gen_module(script)
        #print(Pprint().visit(m) + '\n')
        return compile(m, self.filename, 'exec')

