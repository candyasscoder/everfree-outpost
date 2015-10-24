from collections import namedtuple
import re

from outpost_data.core import util

from outpost_data.core.script.lex import Lexer


class ParseError(Exception):
    pass


TOKEN = re.compile(r'''
        (?P<word>   [a-zA-Z_0-9/]+ ) |
        (?P<punct>  [()[\]*\-%] ) |
        (?:         \s+ | \#.* )''', re.VERBOSE)

def lex(s, filename):
    return Lexer(TOKEN, s, filename).lex()


Section = namedtuple('Section', ('mode', 'obj_kind', 'ext', 'name', 'entries', 'line', 'col'))
Entry = namedtuple('Entry',
        ('ty', 'name', 'min_count', 'max_count', 'weight', 'chance', 'line', 'col'))

INT_RE = re.compile('[0-9]+')

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
                util.err('%s:%d:%d: python blocks are not supported in loot tables' %
                        (self.filename, t.line, t.col))
                while self.peek().kind != 'py_end':
                    self.take()
                self.take()
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

        ext = ty.endswith('_ext')
        if ext:
            ty = ty[:-len('_ext')]

        mode, _, obj_kind = ty.partition('_')
        if mode not in ('choose', 'multi') or obj_kind not in ('structure', 'item'):
            util.err('%s:%d:%d: unknown section type %r' %
                    (self.filename, t_sect.line, t_sect.col, ty))

        entries = []
        while True:
            t_field = self.peek()
            if t_field.kind == 'eol':
                self.take()
                continue
            if t_field.text == '[' or t_field.kind == 'eof':
                break

            try:
                weight, chance = None, None
                if self.peek().text == '(':
                    weight, chance = self.parse_weight_or_chance()

                min_count, max_count = 1, 1
                if INT_RE.match(self.peek().text):
                    min_count, max_count = self.parse_counts()

                table_ref = self.peek().text == '*'
                if table_ref:
                    self.take_punct('*')

                    if min_count != 1 or max_count != 1:
                        util.error('%s:%d:%d: cannot specify count for table reference' %
                                (self.filename, t_field.line, t_field.col))

                ref_name = self.take_word()
                self.take_eol()

                entries.append(Entry('table' if table_ref else 'object',
                    ref_name, min_count, max_count, weight, chance,
                    t_field.line, t_field.col))
            except ParseError:
                self.skip_to_eol()

        return Section(mode, obj_kind, ext, name, entries, t_sect.line, t_sect.col)

    def parse_weight_or_chance(self):
        weight, chance = None, None

        self.take_punct('(')
        amount = self.take_int()
        if self.peek().text == '%':
            self.take_punct('%')
            chance = amount
        else:
            weight = amount
        self.take_punct(')')

        return weight, chance

    def parse_counts(self):
        min_count = self.take_int()
        max_count = min_count

        if self.peek().text == '-':
            self.take_punct('-')
            max_count = self.take_int()

        return min_count, max_count
