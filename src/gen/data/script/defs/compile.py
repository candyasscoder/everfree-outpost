import ast
import textwrap

from outpost_data.core.script.defs.field import FIELD_MAP
from outpost_data.core.script.defs.parse import Section, PythonBlock
from outpost_data.core.script.defs.parse import Backticked, Value

def mk_emit():
    stmts = []
    def emit(s, pos_src=None):
        if isinstance(s, ast.stmt):
            pass
        elif isinstance(s, ast.expr):
            s = ast.Expr(s)
        elif isinstance(s, ExprBuilder):
            s = ast.Expr(s.expr)
        else:
            assert False, 'expected stmt or expr'

        if pos_src is not None:
            s.lineno = pos_src.line
            s.col_offset = pos_src.col

        stmts.append(s)
    return stmts, emit

def val_to_expr(val):
    """Convert a simple Python value to an AST that produces that value."""
    if isinstance(val, ast.AST):
        return val
    elif type(val) is tuple:
        return ast.Tuple([val_to_expr(x) for x in val], ast.Load())
    elif type(val) is int:
        return ast.Num(val)
    elif type(val) is str:
        return ast.Str(val)
    else:
        assert False, \
                "can't convert %r (of type %s) to an AST" % (val, type(val))

def expr(e):
    """Convert most anything to an `ast.expr`."""
    if isinstance(e, ExprBuilder):
        return e.expr
    elif isinstance(e, ast.AST):
        return e
    else:
        return val_to_expr(e)

class ExprBuilder(object):
    def __init__(self, expr):
        self.expr = expr

    def attr(self, name, ctx=None):
        ctx = ctx or ast.Load()
        return ExprBuilder(ast.Attribute(self.expr, name, ctx))

    def index(self, key, ctx=None):
        ctx = ctx or ast.Load()
        key = expr(key)
        return ExprBuilder(ast.Subscript(self.expr, ast.Index(expr(key)), ctx))

    def call(self, *args, **kwargs):
        args_ast = [expr(a) for a in args]
        kwargs_ast = [ast.keyword(k, expr(v)) for k,v in kwargs.items()]
        return ExprBuilder(ast.Call(func=self.expr, args=args_ast, keywords=kwargs_ast))

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

def fix_fields(x, fields):
    for f in fields:
        if not hasattr(x, f):
            setattr(x, f, [])

class FuncFix(ast.NodeVisitor):
    def visit_Call(self, n):
        fix_fields(n, ('args', 'keywords'))
        self.generic_visit(n)

    def visit_FunctionDef(self, n):
        fix_fields(n, ('decorator_list',))
        self.generic_visit(n)

    def visit_arguments(self, n):
        fix_fields(n, ('args', 'kwonlyargs', 'kw_defaults', 'defaults'))
        self.generic_visit(n)

class Pprint(ast.NodeVisitor):
    """Basic AST pretty-printer, for debugging code generation."""

    def visit_Call(self, n):
        func = self.visit(n.func)
        args = tuple(self.visit(a) for a in n.args)
        kwargs = tuple('%s=%s' % (kw.arg, self.visit(kw.value)) for kw in n.keywords)
        return '%s(%s)' % (func, ', '.join(args + kwargs))

    def visit_Name(self, n):
        return n.id

    def visit_Subscript(self, n):
        return '%s[%s]' % (self.visit(n.value), self.visit(n.slice))

    def visit_Index(self, n):
        return self.visit(n.value)

    def visit_Attribute(self, n):
        return '%s.%s' % (self.visit(n.value), n.attr)

    def visit_Tuple(self, n):
        return '(%s)' % (', '.join(self.visit(x) for x in n.elts),)

    def visit_Str(self, n):
        return repr(n.s)

    def visit_Num(self, n):
        return repr(n.n)

    def visit_Assign(self, n):
        lhs = ', '.join(self.visit(x) for x in n.targets)
        rhs = self.visit(n.value)
        return '%s = %s' % (lhs, rhs)

    def visit_Expr(self, n):
        return self.visit(n.value)

    def visit_FunctionDef(self, n):
        args = self.visit(n.args)
        body = '\n'.join(self.visit(x) for x in n.body)
        return 'def %s(%s):\n%s' % (n.name, args, textwrap.indent(body, '    '))

    def visit_ImportFrom(self, n):
        names = ', '.join(self.visit(x) for x in n.names)
        return 'from %s import %s' % (n.module, names)

    def visit_alias(self, n):
        if n.asname is not None:
            return '%s as %s' % (n.name, n.asname)
        else:
            return n.name

    def visit_Module(self, n):
        return '\n'.join(self.visit(x) for x in n.body)

    def generic_visit(self, n):
        return '{%s}' % ast.dump(n)
