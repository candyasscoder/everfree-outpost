import builtins
from collections import namedtuple
import os
import platform
import re
import shlex
import shutil
import subprocess

win32 = platform.system() == 'Windows'

Binding = namedtuple('Binding', ('name', 'value'))
Rule = namedtuple('Rule', ('name', 'bindings'))
Build = namedtuple('Build', ('rule', 'outputs', 'inputs', 'implicit', 'order_only', 'bindings'))
Default = namedtuple('Default', ('targets',))




LINE_CONT = re.compile(r'\$\n[ \t]*')
NON_SPACE = re.compile(r'\S')
MULTI_NON_SPACE = re.compile(r'\S+')


TOKEN_STOP = re.compile(r'(\$|:|\|\||\||=|\s+)')
# $abc or ${abc}.  Either group 1 or 2 is the var name, depending on the variant.
VAR = re.compile(r'(?:\$([a-zA-Z_][a-zA-Z0-9_]*)|\$\{([a-zA-Z_][a-zA-Z0-9_]*)\})')
PIPES = re.compile(r'\|\|?')

def tokenize(s, keep_space=False):
    m = NON_SPACE.search(s)
    if m is None:
        return (0, [])
    else:
        indent = m.start()
        i = m.start()

    tokens = []
    cur_tok = ''
    while True:
        m = TOKEN_STOP.search(s, i)

        if m is None:
            cur_tok += s[i:]
            if cur_tok != '':
                tokens.append(cur_tok)
            return (indent, tokens)
        else:
            cur_tok += s[i:m.start()]
            stop = m.group()

        if m.group() == '$':
            # '$' may actually appear in the interior of a token.
            if m.start() + 1 >= len(s):
                raise ValueError('parse error: $ at end of input')
            next_char = s[m.start() + 1]

            var_m = VAR.match(s, m.start())
            if var_m is not None:
                cur_tok += '${%s}' % (var_m.group(1) or var_m.group(2))
                i = var_m.end()
            else:
                cur_tok += next_char
                i = m.end() + 1
        else:
            # ':', '|', ' ' all actually end the current token, unlike '$'.
            if cur_tok != '':
                tokens.append(cur_tok)
            cur_tok = ''

            if stop.strip() != '' or keep_space:
                tokens.append(stop)

            i = m.end()


class Parser(object):
    def __init__(self, s):
        self.lines = LINE_CONT.sub('', s).splitlines()
        self.pos = 0

    def peek_raw(self):
        m = MULTI_NON_SPACE.search(self.lines[pos])
        if m is None:
            return (None, len(self.lines[pos]))
        else:
            return (m.group(), m.start())

    def peek(self):
        while self.pos < len(self.lines):
            m = MULTI_NON_SPACE.search(self.lines[self.pos])
            if m is None or m.group().startswith('#'):
                self.pos += 1
            else:
                return m.group()
        return None

    def peek_indent(self):
        self.peek() # advance to next interesting line
        if self.pos >= len(self.lines):
            return 0
        return MULTI_NON_SPACE.search(self.lines[self.pos]).start()



    def take(self):
        line = self.lines[self.pos]
        self.pos += 1
        return line


    def parse(self):
        directives = []
        while self.pos < len(self.lines):
            directives.append(self.parse_one())
        return directives

    def parse_one(self):
        word = self.peek()
        if word is None:
            return None
        elif word == 'rule':
            return self.parse_rule()
        elif word == 'build':
            return self.parse_build()
        elif word == 'default':
            return self.parse_default()
        else:
            return self.parse_binding()

    def parse_rule(self):
        indent, tokens = tokenize(self.take())
        if len(tokens) != 2:
            raise ValueError('unexpected tokens after rule name')
        _, name = tokens

        bindings = self.parse_binding_list(indent)
        return Rule(name, bindings)

    def parse_build(self):
        indent, tokens = tokenize(self.take())
        parts = {'outputs': [], 'inputs': [], 'implicit': [], 'order_only': []}
        rule = None
        mode = 'outputs'

        for t in tokens[1:]:
            if t == ':':
                if mode != 'outputs':
                    raise ValueError('expected only one : in build line')
                mode = 'rule'
            elif t == '|':
                mode = 'implicit'
            elif t == '||':
                mode = 'order_only'
            elif mode == 'rule':
                rule = t
                mode = 'inputs'
            else:
                parts[mode].append(t)

        if rule is None:
            raise ValueError('missing rule in build line')

        bindings = self.parse_binding_list(indent)
        return Build(rule,
                parts['outputs'],
                parts['inputs'],
                parts['implicit'],
                parts['order_only'],
                bindings)

    def parse_default(self):
        indent, tokens = tokenize(self.take())
        return Default(tokens[1:])

    def parse_binding(self):
        indent, tokens = tokenize(self.take(), keep_space=True)

        name = tokens[0]
        idx = tokens.index('=')
        if idx == -1:
            raise ValueError('missing = in binding')

        # Eat whitespace between the '=' and the value.
        idx += 1
        while idx < len(tokens) and tokens[idx].strip() == '':
            idx += 1

        val = ''.join(tokens[idx:])

        return Binding(name, val)

    def parse_binding_list(self, indent):
        bindings = []
        while self.peek_indent() > indent:
            bindings.append(self.parse_binding())
        return bindings


ExRule = namedtuple('ExRule', ('scope', 'bindings'))
ExBuild = namedtuple('ExBuild', ('deps', 'file_deps', 'order_only', 'command'))

def subst_vars(s, scope):
    def repl(m):
        name = m.group(1) or m.group(2)
        return scope.get(name, '')
    return VAR.sub(repl, s)

def expand(directives):
    scope = {}
    rules = {}
    builds = {}
    roots = set()

    for d in directives:
        if isinstance(d, Binding):
            name = d.name
            value = subst_vars(d.value, scope)
            scope[name] = value
        elif isinstance(d, Rule):
            name = subst_vars(d.name, scope)
            rules[name] = ExRule(scope.copy(), d.bindings)
        elif isinstance(d, Build):
            rule_name = subst_vars(d.rule, scope) 
            outputs = [subst_vars(p, scope) for p in d.outputs]
            inputs = [subst_vars(p, scope) for p in d.inputs]
            implicit = [subst_vars(p, scope) for p in d.implicit]
            order_only = [subst_vars(p, scope) for p in d.order_only]

            rule = rules.get(rule_name)
            if rule is None:
                raise ValueError('no such rule: %s' % rule_name)

            rule_scope = rule.scope.copy()
            for b in d.bindings:
                name = b.name
                value = subst_vars(b.value, scope)
                rule_scope[name] = value

            rule_scope['in'] = ' '.join(inputs)
            rule_scope['in_newline'] = '\n'.join(inputs)
            rule_scope['out'] = ' '.join(outputs)

            params = {}
            for b in rule.bindings:
                name = b.name
                value = subst_vars(b.value, rule_scope)
                params[name] = value

            deps = tuple(inputs + implicit)
            file_deps = read_depfile(params['depfile']) if 'depfile' in params else ()
            order_only = tuple(order_only)

            build = ExBuild(deps, file_deps, order_only, params['command'])
            for o in outputs:
                builds[o] = build
        elif isinstance(d, Default):
            for t in d.targets:
                path = subst_vars(t, scope)
                roots.add(path)

    return (builds, roots or auto_roots(builds))

LINE_CONT_SLASH = re.compile(r'\\\n\s*')
def read_depfile(path):
    try:
        with open(path, 'r') as f:
            content = f.read()
        lines = LINE_CONT_SLASH.sub(' ', content).splitlines()
        _, _, deps = lines[0].partition(':')
        return tuple(deps.split())
    except IOError:
        return ()

def auto_roots(builds):
    defined = set(builds.keys())
    used = set(dep
            for b in builds.values()
            for dep_list in [b.deps, b.file_deps, b.order_only]
            for dep in dep_list)
    return defined - used


def build(target, builds, missing_ok=False):
    b = builds.get(target)

    if b is None:
        if os.path.exists(target) or missing_ok:
            return
        else:
            raise ValueError('%s does not exist and there is no way to build it' % target)

    for d in b.deps:
        build(d, builds)
    for d in b.file_deps:
        build(d, builds, missing_ok=True)
    for d in b.order_only:
        build(d, builds)

    if not os.path.exists(target):
        need_build = True
    else:
        target_time = os.path.getmtime(target)
        dep_time = target_time - 1
        for d in b.deps:
            dep_time = max(dep_time, os.path.getmtime(d))
        for d in b.file_deps:
            if os.path.exists(d):
                dep_time = max(dep_time, os.path.getmtime(d))

        need_build = dep_time >= target_time

    if need_build:
        print(' >>> %s ' % b.command)
        try:
            os.makedirs(os.path.dirname(target))
        except OSError:
            pass
        run(b.command)

def run(command):
    tokens = shlex.split(command) if not win32 else command.split()
    if tokens[0] == 'cp':
        do_cp(tokens[1:])
    elif tokens[0] == 'touch':
        do_touch(tokens[1:])
    else:
        subprocess.check_call(command, shell=True)

def do_cp(args):
    force = False
    files = []
    for a in args:
        if a.startswith('-'):
            if a == '-f':
                force = True
            else:
                raise ValueError('unsupported option for `cp`: %r' % a)
        else:
            files.append(a)

    assert len(files) == 2, 'wrong number of arguments for `cp`'
    src, dest = files

    if force:
        try:
            os.remove(dest)
        except OSError:
            pass
    shutil.copy(src, dest)

def do_touch(args):
    assert len(args) == 1, 'wrong number of arguments for `touch`'
    path = args[0]
    assert not path.startswith('-'), 'unsupported option for `touch`: %r' % path
    try:
        os.utime(path)
    except OSError:
        with open(path, 'w') as f:
            pass


if __name__ == '__main__':
    from pprint import pprint
    p = Parser(open('build.ninja').read())
    directives = p.parse()
    builds, roots = expand(directives)

    for t in roots:
        build(t, builds)
