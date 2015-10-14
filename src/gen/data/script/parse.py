from collections import namedtuple
import re

from .. import util

from .field import FIELD_MAP


TOKEN = re.compile(r'''
        (?P<word>   [a-zA-Z_0-9/]+ ) |
        (?P<punct>  [()[\]:,] ) |
        (?:         "(?P<quoted> [^"]*)" ) |
        (?:         `(?P<backticked> [^`]*)` ) |
        (?P<ignore> (?: \s+ | \#.* ) )''', re.VERBOSE)

PYTHON_DELIM = '%%%'

Token = namedtuple('Token', ('kind', 'text', 'line', 'col'))

class ParseError(Exception):
    pass

class Lexer(object):
    def __init__(self, s, filename):
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
                m = TOKEN.match(line, self.j)
                if m is None:
                    text = line[self.j : self.j + 10]
                    self.err('invalid token starting "%s..."' % text)
                    return None

                word, punct, quoted, backticked, ignore = m.groups()
                if word is not None:
                    self.emit('word', word)
                elif punct is not None:
                    self.emit('punct', punct)
                elif quoted is not None:
                    self.emit('quoted', quoted)
                elif backticked is not None:
                    self.emit('backticked', backticked)
                elif ignore is not None:
                    pass
                else:
                    assert False, 'match succeeded but captured no groups?'

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

def lex(s, filename):
    return Lexer(s, filename).lex()

PythonBlock = namedtuple('InlinePython', ('code', 'line', 'col'))
Section = namedtuple('Section', ('ty', 'name', 'fields', 'line', 'col'))
Field = namedtuple('Field', ('name', 'value', 'line', 'col'))
Backticked = namedtuple('Backticked', ('src', 'line', 'col'))
Value = namedtuple('Value', ('value', 'line', 'col'))

class Parser(object):
    def __init__(self, tokens, filename):
        self.pos = 0
        self.tokens = tokens
        self.filename = filename

    def peek(self):
        return self.tokens[self.pos]

    def take(self):
        t = self.tokens[self.pos]
        self.pos += 1
        return t

    def skip_to_eol(self):
        # Be careful with this one.  It's used to recover from parse errors, so
        # it might be called while the parser is in a weird state.
        while self.pos < len(self.tokens) - 1 and self.tokens[self.pos].kind != 'eol':
            self.pos += 1
        if self.tokens[self.pos].kind == 'eol':
            self.pos += 1

    def take_word(self, expected=None):
        t = self.take()
        if t.kind != 'word':
            self.error(expected or 'word', t)
        return t.text

    def take_int(self, expected=None):
        t = self.take()
        if t.kind == 'word':
            try:
                # Successful path
                return int(t.text)
            except ValueError:
                pass
        # Something went wrong.
        self.error(expected or 'integer', t)


    def take_punct(self, text, expected=None):
        t = self.take()
        if t.kind != 'punct' or t.text != text:
            self.error(expected or '"%s"' % text, t)

    def take_quoted(self, expected=None):
        t = self.take()
        if t.kind != 'quoted':
            self.error(expected or 'quoted string', t)
        return t.text

    def take_eol(self, expected=None):
        t = self.take()
        if t.kind != 'eol':
            self.error(expected or 'end of line', t)

    def take_py_end(self, expected=None):
        t = self.take()
        if t.kind != 'py_end':
            self.error(expected or 'end of Python block', t)

    def error(self, expected, token=None):
        token = token or self.peek()
        text = token.text
        if len(text) > 13:
            text = text[:10] + '...'
        util.err('%s:%d:%d: parse error: expected %s, but saw %s "%s"' %
                (self.filename, token.line, token.col, expected, token.kind, text))
        raise ParseError()

    def parse(self):
        parts = []
        while True:
            t = self.peek()
            if t.kind == 'eol':
                self.take()
            elif t.kind == 'eof':
                break
            elif t.kind == 'punct' and t.text == '[':
                parts.append(self.parse_section())
            elif t.kind == 'py_begin':
                parts.append(self.parse_python_block())
            else:
                self.error('beginning of section', t)
        return parts

    def parse_section(self):
        t_sect = self.peek()
        self.take_punct('[')
        ty = self.take_word('section type')
        name = self.take_word('section name')
        self.take_punct(']')
        self.take_eol()

        if ty in FIELD_MAP:
            field_map = FIELD_MAP[ty]
        else:
            util.err('%s:%d:%d: unknown section type %r' %
                    (self.filename, t_sect.line, t_sect.col, ty))
            field_map = {}
            ty = None

        parts = []
        while True:
            t_field = self.peek()
            if self.peek().kind != 'word':
                break

            try:
                key = self.take_word()
                self.take_punct(':')

                if key not in field_map:
                    if ty is not None:
                        util.err('%s:%d:%d: unknown field %r for section type %r' %
                                (self.filename, t_field.line, t_field.col, key, ty))
                    raise ParseError()

                if self.peek().kind == 'backticked':
                    t_val = self.take()
                    val = Backticked(t_val.text, t_val.line, t_val.col)
                else:
                    t_val = self.peek()
                    val = Value(field_map[key].parse(self), t_val.line, t_val.col)

                self.take_eol()

                parts.append(Field(key, val, t_field.line, t_field.col))
            except ParseError as e:
                self.skip_to_eol()

        return Section(ty, name, parts, t_sect.line, t_sect.col)

    def parse_python_block(self):
        begin = self.take()
        code = []
        while self.peek().kind == 'py_line':
            t = self.take()
            code.append(t.text)
        end = self.take_py_end()

        return PythonBlock('\n'.join(code), begin.line, begin.col)

    parse_filename = lambda self: self.take_quoted('quoted filename')
