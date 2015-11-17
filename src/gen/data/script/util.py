import ast
import textwrap


def mk_emit():
    """Produce a reference to an empty list of statements and a function to add
    a statement to the list."""
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
    elif type(val) is bool:
        return ast.Name('True' if val else 'False', ast.Load())
    elif val is None:
        return ast.Name('None', ast.Load())
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
