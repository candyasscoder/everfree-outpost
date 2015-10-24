from outpost_data.core.script.loot import parse, compile

Compiler = compile.Compiler

def parse_script(text, filename='<string>'):
    tokens = parse.lex(text, filename)
    parser = parse.Parser(tokens, filename)
    return parser.parse()

