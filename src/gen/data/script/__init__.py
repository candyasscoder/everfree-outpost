from . import parse, interp

Interpreter = interp.Interpreter

def parse_script(text, filename='<string>'):
    tokens = parse.lex(text, filename)
    parser = parse.Parser(tokens, filename)
    return parser.parse()
