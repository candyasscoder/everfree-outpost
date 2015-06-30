import builtins
import re
import textwrap


BLOCK_FOR = re.compile(r'^[ \t]*%(for [^%\n]*)$', re.MULTILINE)
BLOCK_IF = re.compile(r'^[ \t]*%(if [^%\n]*)$', re.MULTILINE)
BLOCK_ELIF = re.compile(r'^[ \t]*%(elif [^%\n]*)$', re.MULTILINE)
BLOCK_ELSE = re.compile(r'^[ \t]*%(else)[ \t]*$', re.MULTILINE)
BLOCK_END = re.compile(r'^[ \t]*%(end)[ \t]*$', re.MULTILINE)

FOR_PARTS = re.compile(r'for ([a-zA-Z0-9_]*(?: *, *[a-zA-Z0-9_]*)*) in (.*)$')

PLACEHOLDER = re.compile(r'(%([a-zA-Z0-9_]+)\b|%{([^}\n]*)})')

def lines(s):
    i = 0
    while i < len(s):
        end = s.find('\n', i)
        if end == -1:
            end = len(s)
        else:
            end += 1

        yield i, end
        i = end

class TemplateRender(object):
    def __init__(self, s, kwargs):
        self.s = s
        self.args = kwargs

    def render(self):
        out = ''
        depth = 0
        header = None
        header_line = 0
        blocks = []
        for line_num, (start, end) in enumerate(lines(self.s)):
            m = None
            def match(r):
                nonlocal m
                m = r.match(self.s, start)
                return m

            if match(BLOCK_FOR) or match(BLOCK_IF):
                if depth == 0:
                    header = m
                    header_line = line_num
                    blocks = []
                depth += 1
            elif match(BLOCK_ELSE) or match(BLOCK_ELIF):
                if depth == 0:
                    raise ValueError('bad syntax: stray %r on line %d' % (m.group(1), line_num))
                elif depth == 1:
                    blocks.append((header, header_line, header.end() + 1, m.start()))
                    header_line = line_num
                    header = m
            elif match(BLOCK_END):
                if depth == 0:
                    raise ValueError('bad syntax: stray %r on line %d' % (m.group(1), line_num))
                elif depth == 1:
                    blocks.append((header, header_line, header.end() + 1, m.start()))
                    out += self._do_block(blocks)
                    plain_start = m.end() + 1
                depth -= 1
            else:
                if depth == 0:
                    out += self._do_plain(start, end)
        return out

    def _do_block(self, parts):
        h, h_line, start, end = parts[0]
        if h.group(1).startswith('for'):
            if len(parts) != 1:
                raise ValueError('bad syntax: unclosed %%for on line %d' % h_line)
            m = FOR_PARTS.match(h.group(1))
            if not m:
                raise ValueError('bad syntax: invalid %%for on line %d' % h_line)

            var_names = [v.strip() for v in m.group(1).split(',')]
            collection = eval(m.group(2), {'__builtins__': builtins}, self.args)
            dct = self.args.copy()
            out = ''
            for x in collection:
                if len(var_names) == 1:
                    dct[var_names[0]] = x
                else:
                    if len(var_names) != len(x):
                        raise ValueError('line %d: wrong number of values to unpack' % h_line)
                    for name, val in zip(var_names, x):
                        dct[name] = val
                out += TemplateRender(self.s[start:end], dct).render()
            return out
        else:   # `%if`
            for i, (h, h_line, start, end) in enumerate(parts):
                if h.group(1) == 'else' and i != len(parts) - 1:
                    raise ValueError('bad syntax: more cases after %%else on line %d' % h_line)
            for h, h_line, start, end in parts:
                if h.group(1) == 'else':
                    go = True
                else:
                    cond = h.group(1).partition(' ')[2]
                    go = eval(cond, {'__builtins__': builtins}, self.args)
                if go:
                    return TemplateRender(self.s[start:end], self.args).render()
            return ''

    def _do_plain(self, start, end):
        def repl(m):
            expr = m.group(2) or m.group(3)
            if expr:
                return eval(expr, {'__builtins__': builtins}, self.args)
            else:
                # Use '%{}' to produce a literal '%'
                return '%'
        line = self.s[start:end]
        if line.endswith('%\n'):
            line = line[:-2]
        return PLACEHOLDER.sub(repl, line)

PREP_RE = re.compile(r'%(for|if|elif|else|end)\b[^%\n]*%')
def template(s, **kwargs):
    s = textwrap.dedent(s).strip('\n')
    # Turn inline %if/%for into multiline ones
    def repl(m):
        return '%\n' + m.group(0)[:-1] + '\n'
    s = PREP_RE.sub(repl, s)
    return TemplateRender(s, kwargs).render()
