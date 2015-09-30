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

Token = namedtuple('Token', ('kind', 'text', 'line', 'col'))

class ParseError(Exception):
    pass

def lex(s, filename):
    lines = s.splitlines()

    tokens = []
    def emit(kind, text, line, col):
        tokens.append(Token(kind, text, line, col))

    for i, line in enumerate(lines):
        j = 0
        while j < len(line):
            m = TOKEN.match(line, j)
            if m is None:
                text = line[j : j + 10]
                util.err('%s:%d:%d: invalid token starting "%s..."' %
                        (filename, i + 1, j + 1, text))
                return None

            word, punct, quoted, backticked, ignore = m.groups()
            if word is not None:
                emit('word', word, i + 1, j + 1)
            elif punct is not None:
                emit('punct', punct, i + 1, j + 1)
            elif quoted is not None:
                emit('quoted', quoted, i + 1, j + 1)
            elif backticked is not None:
                emit('backticked', backticked, i + 1, j + 1)
            elif ignore is not None:
                pass
            else:
                assert False, 'match succeeded but captured no groups?'

            j = m.end()

        emit('eol', '', i + 1, j + 1)

    emit('eof', '', i + 2, 1)

    return tokens

Section = namedtuple('Section', ('ty', 'name', 'fields'))
Field = namedtuple('Field', ('name', 'value'))
Backticked = namedtuple('Backticked', ('src',))
Value = namedtuple('Value', ('value',))

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

    def error(self, expected, token=None):
        token = token or self.peek()
        text = token.text
        if len(text) > 13:
            text = text[:10] + '...'
        util.err('%s:%d:%d: parse error: expected %s, but saw %s "%s"' %
                (self.filename, token.line, token.col, expected, token.kind, text))
        raise ParseError()

    def parse(self):
        sections = []
        while True:
            t = self.peek()
            if t.kind == 'eol':
                self.take()
            elif t.kind == 'eof':
                break
            elif t.kind == 'punct' and t.text == '[':
                sections.append(self.parse_section())
            else:
                self.error('beginning of section', t)
        return sections

    def parse_section(self):
        self.take_punct('[')
        t = self.peek()
        ty = self.take_word('section type')
        name = self.take_word('section name')
        self.take_punct(']')
        self.take_eol()

        if ty in FIELD_MAP:
            field_map = FIELD_MAP[ty]
        else:
            util.err('%s:%d:%d: unknown section type %r' %
                    (self.filename, t.line, t.col, ty))
            field_map = {}
            ty = None

        parts = []
        while True:
            t = self.peek()
            if self.peek().kind != 'word':
                break

            try:
                key = self.take_word()
                self.take_punct(':')

                if key not in field_map:
                    if ty is not None:
                        util.err('%s:%d:%d: unknown field %r for section type %r' %
                                (self.filename, t.line, t.col, key, ty))
                    raise ParseError()

                if self.peek().kind == 'backticked':
                    val = Backticked(self.take().text)
                else:
                    val = Value(field_map[key].parse(self))

                self.take_eol()

                parts.append(Field(key, val))
            except ParseError as e:
                self.skip_to_eol()

        return Section(ty, name, parts)
        
    parse_filename = lambda self: self.take_quoted('quoted filename')

    def parse_item_count(self):
        count = self.take_int()
        item = self.take_word()
        return (count, item)

    def parse_shape(self):
        kind = self.take_word()
        self.take_punct('(')
        x = self.take_int()
        self.take_punct(',')
        y = self.take_int()
        self.take_punct(',')
        z = self.take_int()
        self.take_punct(')')
        return (kind, (x, y, z))
