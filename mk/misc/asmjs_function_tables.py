import json
import sys

in_metadata = False
code_str = ''
metadata_str = ''
for line in sys.stdin:
    if line.strip() == '// EMSCRIPTEN_METADATA':
        in_metadata = True
        continue

    if in_metadata:
        metadata_str += line
    else:
        code_str += line

metadata = json.loads(metadata_str)

def make_cast(val, ty):
    if ty == 'i':
        return '%s|0' % val;
    elif ty == 'd':
        return '+%s' % val
    elif ty == 'f':
        return 'fround(%s)' % val
    elif ty == 'v':
        return ''
    else:
        raise ValueError('unknown type: %s' % ty)

def make_abort_func(sig, name):
    ret_ty = sig[0]
    arg_tys = sig[1:]
    args = ', '.join('$%d' % i for i in range(len(arg_tys)))
    body = ' '.join('$%d = %s;' % (i, make_cast('$%d' % i, arg_tys[i]))
            for i in range(len(arg_tys)))
    ret = make_cast(0, ret_ty)
    return 'function %s(%s) { %s abort(); return %s; }' % (name, args, body, ret)

abort_funcs_str = ''
fn_tables_str = ''

for k, v in metadata['tables'].items():
    start = v.index('[')
    end = v.index(']')
    funcs = v[start + 1 : end].split(',')

    # Size is always a power of two, so the bit mask is size - 1.
    code_str = code_str.replace('#FM_%s#' % k, str(len(funcs) - 1))

    abort_name = '__abort_%s' % k
    abort_func = make_abort_func(k, abort_name)
    abort_funcs_str += abort_func + '\n'

    fn_table_body = ',\n    '.join(f if f != '0' else abort_name for f in funcs)
    fn_tables_str += 'var FUNCTION_TABLE_%s = [\n    %s\n  ];\n' % (k, fn_table_body)

END_MARKER = '\n// EMSCRIPTEN_END_FUNCTIONS\n'
code_str = code_str.replace(END_MARKER,
        '\n\n// ABORT FUNCTIONS:\n\n%s' % abort_funcs_str +
        '\n\n// FUNCTION TABLES:\n\n%s' % fn_tables_str +
        '\n' + END_MARKER)

print(code_str)


