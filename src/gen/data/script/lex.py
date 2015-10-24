from collections import namedtuple

from outpost_data.core import util

PYTHON_DELIM = '%%%'

Token = namedtuple('Token', ('kind', 'text', 'line', 'col'))

class Lexer(object):
    def __init__(self, token_re, s, filename):
        self.token_re = token_re
        self.filename = filename
        self.lines = s.splitlines()
        self.i = 0
        self.j = 0
        self.tokens = []

    def emit(self, kind, text):
        self.tokens.append(Token(kind, text, self.i + 1, self.j))

    def err(self, msg):
        util.err('%s:%d:%d: %s' % (self.filename, self.i + 1, self.j, msg))

    def next_line(self):
        self.i += 1
        self.j = 0

    def lex_normal(self):
        while self.i < len(self.lines):
            line = self.lines[self.i]
            if line == PYTHON_DELIM:
                self.emit('py_begin', line)
                self.next_line()
                return 'python'

            while self.j < len(line):
                m = self.token_re.match(line, self.j)
                if m is None:
                    text = line[self.j : self.j + 10]
                    self.err('invalid token starting "%s..."' % text)
                    return None

                # NB: assumes at most one named group will match
                for k,v in m.groupdict().items():
                    if v is not None:
                        self.emit(k, v)
                        break

                self.j = m.end()

            self.emit('eol', '')
            self.next_line()
        return 'done'

    def lex_python(self):
        start = self.i - 1
        while self.i < len(self.lines):
            line = self.lines[self.i]
            if line == PYTHON_DELIM:
                self.emit('py_end', line)
                self.next_line()
                return 'normal'

            self.emit('py_line', line)
            self.next_line()

        self.err('unclosed Python code block starting at line %d' % start)
        return None

    def lex(self):
        state = 'normal'
        while True:
            if state == 'normal':
                state = self.lex_normal()
            elif state == 'python':
                state = self.lex_python()
            elif state == 'done':
                self.emit('eof', '')
                return self.tokens
            elif state is None:
                return None
