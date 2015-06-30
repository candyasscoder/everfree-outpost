import textwrap


def mk_bindings(**kwargs):
    return '\n'.join('%s = %s' % (k, v) for k,v in kwargs.items())

def mk_rule(name, **kwargs):
    bindings_str = mk_bindings(**kwargs)
    return 'rule %s\n' % name + textwrap.indent(bindings_str, '  ')

def mk_build(targets, rule, inputs, implicit=(), order=(), **kwargs):
    targets = targets if isinstance(targets, str) else ' '.join(targets)
    inputs = inputs if isinstance(inputs, str) else ' '.join(inputs)
    implicit = ' | %s' % ' '.join(implicit) if implicit else ''
    order = ' || %s' % ' '.join(order) if order else ''
    bindings = '\n' + textwrap.indent(mk_bindings(**kwargs), '  ') if len(kwargs) > 0 else ''
    return 'build %s: %s %s%s%s%s' % (targets, rule, inputs, implicit, order, bindings)

def maybe(s, x, t=''):
    if x is not None:
        return s % x
    else:
        return t

def cond(x, a, b=''):
    if x:
        return a
    else:
        return b

def join(*args):
    return ' '.join(args)
