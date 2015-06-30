import argparse
import builtins
import os
import re
import subprocess
import sys
import textwrap

def build_parser():
    args = argparse.ArgumentParser()

    args.add_argument('--build-dir', default=None,
            help='directory to store build files')
    args.add_argument('--dist-dir', default=None,
            help='directory to store distribution image')

    args.add_argument('--debug', action='store_true',
            help='produce a debug build')
    args.add_argument('--release', action='store_false', dest='debug',
            help='produce a release build (default)')

    args.add_argument('--rust-home', default='../rust',
            help='path to rust-lang/rust checkout')
    args.add_argument('--bitflags-home', default='../bitflags',
            help='path to rust-lang/bitflags checkout')
    args.add_argument('--rust-extra-libdir', default=None,
            help='additional search directory for Rust libraries')
    args.add_argument('--emscripten-fastcomp-prefix', default=None,
            help='path to kripken/emscripten-fastcomp build/install directory')
    args.add_argument('--emscripten-passes-prefix', default=None,
            help='path to epdtry/rust-emscripten-passes build directory')

    args.add_argument('--rustc', default='rustc',
            help='name of the Rust compiler binary')

    return args


class Info(object):
    def __init__(self, args):
        self._args = args
        self.src_dir = os.path.dirname(sys.argv[0]) or '.'

        in_tree = self.src_dir == '.' or self.src_dir == os.getcwd()

        if args.build_dir is None:
            self.build_dir = 'build' if in_tree else '.'
        else:
            self.build_dir = args.build_dir

        if args.dist_dir is None:
            self.dist_dir = 'dist' if in_tree else os.path.join(self.build_dir, 'dist')
        else:
            self.dist_dir = args.dist_dir

    def __getattr__(self, k):
        return getattr(self._args, k)


BLOCK_FOR = re.compile(r'^[ \t]*%(for [^%\n]*)$', re.MULTILINE)
BLOCK_IF = re.compile(r'^[ \t]*%(if [^%\n]*)$', re.MULTILINE)
BLOCK_ELIF = re.compile(r'^[ \t]*%(elif [^%\n]*)$', re.MULTILINE)
BLOCK_ELSE = re.compile(r'^[ \t]*%(else)[ \t]*$', re.MULTILINE)
BLOCK_END = re.compile(r'^[ \t]*%(end)[ \t]*$', re.MULTILINE)

FOR_PARTS = re.compile(r'for ([a-zA-Z0-9_]*(?: *, *[a-zA-Z0-9_]*)*) in (.*)$')

PLACEHOLDER = re.compile(r'(%([a-zA-Z0-9_]+)\b|%{([a-zA-Z0-9_]*)})')

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
            name = m.group(2) or m.group(3)
            if name:
                return self.args[name]
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


def mk_bindings(**kwargs):
    return template('''
        %for k,v in sorted(args.items())
        %k = %v
        %end
    ''', args=kwargs)

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

def emit_bindings(*args, **kwargs):
    print(mk_bindings(*args, **kwargs))

def emit_rule(*args, **kwargs):
    print(mk_rule(*args, **kwargs))

def emit_build(*args, **kwargs):
    print(mk_build(*args, **kwargs))

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

def emit_header(i):
    emit_bindings(
        src=os.path.normpath(i.src_dir),
        # Note: (1) `build` is a ninja keyword; (2) `builddir` is a special variable that determines
        # where `.ninja_log` is stored.
        builddir=os.path.normpath(i.build_dir),
        dist=os.path.normpath(i.dist_dir),
    )

    def b(*args):
        return os.path.normpath(os.path.join(i.build_dir, *args))

    emit_bindings(
        b_native=b('native'),
        b_asmjs=b('asmjs'),
        b_data=b('data'),
        b_www=b('www'),
        )

    emit_bindings(
        rust_home=i.rust_home,
        bitflags_home=i.bitflags_home,

        rustc=i.rustc,
        python3='python3',
        cc='gcc',
        cxx='g++',
        closure_compiler='closure-compiler',
        yui_compressor='yui-compressor',
        )

    if i.emscripten_fastcomp_prefix is not None:
        base = i.emscripten_fastcomp_prefix
        emit_bindings(
            em_llvm_as=os.path.join(base, 'bin', 'llvm-as'),
            em_opt=os.path.join(base, 'bin', 'opt'),
            em_llc=os.path.join(base, 'bin', 'llc'),
            )
    else:
        emit_bindings(
            em_llvm_as='llvm-as',
            em_opt='opt',
            em_llc='llc',
            )

    if i.emscripten_passes_prefix is not None:
        base = i.emscripten_passes_prefix
        emit_bindings(
            em_pass_remove_overflow_checks=os.path.join(base, 'RemoveOverflowChecks.so'),
            em_pass_remove_assume=os.path.join(base, 'RemoveAssume.so'),
            )
    else:
        emit_bindings(
            em_pass_remove_overflow_checks='RemoveOverflowChecks.so',
            em_pass_remove_assume='RemoveAssume.so',
            )


def mk_native_rules(i):
    cmd_base = join('$rustc $in',
            '--out-dir $b_native',
            '--emit link,dep-info',
            '-L $b_native',
            maybe('-L %s', i.rust_extra_libdir),
            cond(i.debug, '', '-C opt-level=3'),
            maybe('--extern log=%s/liblog.rlib', i.rust_extra_libdir),
            maybe('--extern rand=%s/librand.rlib', i.rust_extra_libdir),
            )

    common_cflags = join(
            '-MMD -MF $out.d',
            '$picflag',
            cond(i.debug, '-ggdb', '-O3'),
            )

    return template('''
        rule rustc_native_bin
            command = %cmd_base --crate-type=bin  %if i.debug% -C lto %end%
            depfile = $b_native/$crate_name.d
            description = RUSTC $out

        rule rustc_native_lib
            command = %cmd_base --crate-type=lib
            depfile = $b_native/$crate_name.d
            description = RUSTC $out

        rule c_obj
            command = $cc -c $in -o $out -std=c99 %common_cflags $cflags
            depfile = $out.d
            description = CC $out

        rule cxx_obj
            command = $cxx -c $in -o $out -std=c++14 %common_cflags $cflags
            depfile = $out.d
            description = CXX $out

        rule link_bin
            command = $cxx $in -o $out $ldflags $libs
            description = LD $out

        rule link_shlib
            command = $cxx -shared $in -o $out $ldflags $libs
            description = LD $out
    ''', **locals())

def mk_rustc_build(crate_name, crate_type, deps, src_file=None):
    if crate_type == 'bin':
        output_name = crate_name
        src_file = src_file or '$src/%s/main.rs' % crate_name
    elif crate_type == 'lib':
        output_name = 'lib%s.rlib' % crate_name
        src_file = src_file or '$src/%s/lib.rs' % crate_name

    return mk_build(
            '$b_native/%s' % output_name,
            'rustc_native_%s' % crate_type,
            src_file,
            ('$b_native/lib%s.rlib' % d for d in deps),
            crate_name=crate_name)

def mk_cxx_build(out_file, out_type, build_dir, src_files, **kwargs):
    builds = []
    def add_build(*args, **kwargs):
        builds.append(mk_build(*args, **kwargs))

    pic_flag = '' if out_type == 'bin' else '-fPIC'

    deps = []
    for f in src_files:
        obj_file = '%s/%s.o' % (build_dir, os.path.basename(f))
        if f.endswith('.c'):
            build_type = 'c_obj'
        else:
            build_type = 'cxx_obj'
        add_build(obj_file, build_type, f, picflag=pic_flag, **kwargs)
        deps.append(obj_file)

    add_build(out_file, 'link_%s' % out_type, deps, **kwargs)

    return '\n'.join(builds)


def mk_asmjs_rules(i):
    compile_base = join('$rustc $in',
            '--out-dir $b_asmjs',
            '--cfg asmjs',
            '--target=i686-unknown-linux-gnu',
            '-L $b_asmjs -L $b_native',
            maybe('-L %s', i.rust_extra_libdir),
            # -C opt-level=3 is mandatory because it eliminates some constructs that cause problems
            # for emscripten-fastcomp.
            '-C opt-level=3',
            '-Z no-landing-pads -C no-stack-check',
            '-C no-vectorize-loops -C no-vectorize-slp')

    return template(r'''
        rule asm_compile_rlib
            command = %compile_base --emit=link,dep-info --crate-type=rlib
            depfile = $b_asmjs/$crate_name.d
            description = RUSTC $out

        rule asm_compile_ir
            # Like opt-level=3 above, lto is mandatory to prevent emscripten-fastcomp errors.
            command = %compile_base --emit=llvm-ir,dep-info --crate-type=staticlib -C lto
            depfile = $b_asmjs/$crate_name.d
            description = RUSTC $out

        rule asm_clean_ir
            command = sed <$in >$out $
                -e 's/\<dereferenceable([0-9]*)//g' $
                -e '/^!/s/\(.\)!/\1metadata !/g' $
                -e '/^!/s/distinct //g'
            description = ASMJS $out

        rule asm_assemble_bc
            command = $em_llvm_as $in -o $out
            description = ASMJS $out


        rule asm_optimize_bc
            command = $em_opt $in $
                -load=$em_pass_remove_overflow_checks $
                -load=$em_pass_remove_assume $
                -strip-debug $
                -internalize -internalize-public-api-list="$$(cat $exports_file)" $
                -remove-overflow-checks $
                -remove-assume $
                -globaldce $
                -pnacl-abi-simplify-preopt -pnacl-abi-simplify-postopt $
                -enable-emscripten-cxx-exceptions $
                -o $out
            description = ASMJS $out

        rule asm_convert_exports
            command = tr '\n' ',' <$in >$out
            description = ASMJS $out

        rule asm_generate_js
            command = $em_llc $in $
                -march=js -filetype=asm $
                -emscripten-assertions=1 $
                -emscripten-no-aliasing-function-pointers $
                -emscripten-max-setjmps=20 $
                -O3 $
                -o $out
            description = ASMJS $out

        rule asm_add_function_tables
            command = $python3 $src/util/asmjs_function_tables.py <$in >$out
            description = ASMJS $out

        rule asm_insert_functions
            command = awk -f $src/util/asmjs_insert_functions.awk <$in >$out
            description = ASMJS $out
    ''', **locals())

def mk_asmjs_rlib(crate_name, deps, src_file=None):
    src_file = src_file or '$src/%s/lib.rs' % crate_name

    return mk_build(
            '$b_asmjs/lib%s.rlib' % crate_name, 
            'asm_compile_rlib',
            src_file,
            ('$b_asmjs/lib%s.rlib' % d for d in deps),
            crate_name=crate_name)

def mk_asmjs_asmlibs(name, rust_src, rust_deps, exports_file, template_file):
    return template('''
        build %base.ll: asm_compile_ir %rust_src $
            | %for d in rust_deps% $b_asmjs/lib%d.rlib %end%
            crate_name = %name
        build %base.clean.ll: asm_clean_ir %base.ll
        build %base.bc: asm_assemble_bc %base.clean.ll
        build %base.exports.txt: asm_convert_exports %exports_file
        build %base.opt.bc: asm_optimize_bc %base.bc | %base.exports.txt
            exports_file = %base.exports.txt
        build %base.0.js: asm_generate_js %base.opt.bc
        build %base.1.js: asm_add_function_tables %base.0.js $
            | $src/util/asmjs_function_tables.py
        build %base.js: asm_insert_functions %template_file $
            | %base.1.js $src/util/asmjs_insert_functions.awk
    ''', base = '$b_asmjs/%s' % name, **locals())


def mk_bitflags_fix(src_in, src_out):
    return template('''
        rule fix_bitflags_src
            command = $
                echo '#![feature(no_std)]' >$out && $
                echo '#![no_std]' >>$out && $
                cat $in >> $out
            description = PATCH bitflags.rs

        build %src_out: fix_bitflags_src %src_in
    ''', **locals())


def mk_js_opt_rules(i):
    return template('''
        rule js_compile_modules
            command = $
                %if not i.debug
                $closure_compiler $
                    $$($python3 $src/util/collect_js_deps.py $in $out $depfile) $
                    --js_output_file=$out $
                    --language_in=ECMASCRIPT5_STRICT $
                    --compilation_level=ADVANCED_OPTIMIZATIONS $
                    --output_wrapper='(function(){%{}output%{}})' $
                    --jscomp_error=undefinedNames $
                    --jscomp_error=undefinedVars $
                    --create_name_map_files $
                    --process_common_js_modules $
                    --common_js_entry_module=$entry_module $
                    --common_js_module_path_prefix=$module_dir $
                    --externs=$src/util/closure_externs.js
                %else
                $python3 $src/util/gen_js_loader.py $
                    $$($python3 $src/util/collect_js_deps.py $in $out $depfile) $
                    >$out
                %end
            description = MIN $out
            depfile = $out.d

        rule js_minify_file
            command = $
                %if not i.debug
                $yui_compressor --disable-optimizations --line-break 200 $
                    $in $filter >$out
                %else
                cp $in $out
                %end
            description = MIN $out
    ''', **locals())

def mk_compile_modules(i, main_src, out_file=None):
    main_dir, basename_ext = os.path.split(main_src)
    module_name, _ = os.path.splitext(basename_ext)
    if out_file is None:
        out_file = '$b_www/%s.js' % module_name

    return template('''
        build %out_file: js_compile_modules %main_src $
            | $src/util/collect_js_deps.py $
              %if i.debug% $src/util/gen_js_loader.py %end%
            entry_module = %module_name
            module_dir = %main_dir
    ''', **locals())

def mk_minify_asm(js_src, out_file=None):
    if out_file is None:
        out_file = '$b_www/%s' % os.path.basename(js_src)

    return template('''
        build %out_file: js_minify_file %js_src
            filter = | sed -e '1s/{/{"use asm";/'
    ''', **locals())


                


def mk_gen_rules(i):
    return template('''
        rule process_font
            command = $python3 $src/util/process_font.py $
                --font-image-in=$in $
                --first-char=$first_char $
                --font-image-out=$out_img $
                --font-metrics-out=$out_metrics
            description = GEN $out_img

        rule process_day_night
            command = $python3 $src/util/gen_day_night.py $in >$out
            description = GEN $out

        rule gen_server_json
            command = $python3 $src/util/gen_server_json.py >$out
            description = GEN $out
    ''')

def mk_font_build(src_img, out_base):
    out_img = out_base + '.png'
    out_metrics = out_base + '_metrics.json'

    return template('''
        build %out_img %out_metrics: process_font %src_img $
            | $src/util/process_font.py
            first_char = 0x21
            out_img = %out_img
            out_metrics = %out_metrics
    ''', **locals())

def mk_day_night_build(src_img, out_json):
    return template('''
        build %out_json: process_day_night %src_img $
            | $src/util/gen_day_night.py
    ''', **locals())

def mk_server_json_build(out_json):
    return template('''
        build %out_json: gen_server_json | $src/util/gen_server_json.py
    ''', **locals())


def mk_data_processing():
    return template('''
        rule process_data
            command = rm -f $b_data/structures*.png && $
                $python3 $src/data/main.py $src/assets $b_data && $
                touch $b_data/stamp
            description = DATA
            depfile = $b_data/data.d

        build $b_data/stamp $
            %for side in ('server', 'client')
                %for file in ('structures', 'blocks', 'items', 'recipes')
                    $b_data/%{file}_%{side}.json $
                %end
            %end
            $b_data/tiles.png $b_data/items.png: $
            process_data | $src/data/main.py
    ''', **locals())

def mk_pack():
    return template('''
        rule build_pack
            command = $python3 $src/util/make_pack.py $src $b_data $b_data/outpost.pack
            description = PACK
            depfile = $b_data/outpost.pack.d

        build $b_data/outpost.pack: build_pack $
            || $b_data/stamp $b_data/font.png $b_data/day_night.json
    ''', **locals())


if __name__ == '__main__':
    parser = build_parser()
    args = parser.parse_args(sys.argv[1:])
    i = Info(args)


    emit_header(i)

    print('\n# Native compilation rules')
    print(mk_native_rules(i))

    print('\n# Asm.js compilation rules')
    print(mk_asmjs_rules(i))

    print('\n# Data processing rules')
    print(mk_gen_rules(i))


    print('\n# Asm.js Rust libraries')
    print(mk_asmjs_rlib('core', (), '$rust_home/src/libcore/lib.rs'))
    print(mk_bitflags_fix('$bitflags_home/src/lib.rs', '$b_asmjs/bitflags.rs'))
    print(mk_asmjs_rlib('bitflags', (), '$b_asmjs/bitflags.rs'))
    print(mk_asmjs_rlib('asmrt', ('core',)))
    print(mk_asmjs_rlib('physics', ('core', 'bitflags', 'asmrt')))
    print(mk_asmjs_rlib('graphics', ('core', 'asmrt', 'physics')))
    print(mk_asmjs_asmlibs('asmlibs',
        '$src/client/asmlibs.rs', ('core', 'asmrt', 'physics', 'graphics'),
        '$src/client/asmlibs_exports.txt', '$src/client/asmlibs.tmpl.js'))

    print('\n# Native Rust binaries')
    print(mk_rustc_build('physics', 'lib', ()))
    print(mk_rustc_build('backend', 'bin', ('physics',), '$src/server/main.rs'))

    print('\n# C/C++ binaries')
    print(mk_cxx_build('$b_native/wrapper', 'bin', '$b_native/wrapper_objs',
        ('$src/wrapper/%s' % f for f in os.listdir(os.path.join(i.src_dir, 'wrapper'))
            if f.endswith('.cpp')),
        cxxflags='-DWEBSOCKETPP_STRICT_MASKING',
        ldflags='-static',
        libs='-lboost_system -lpthread'))

    py_includes = subprocess.check_output(('python3-config', '--includes')).decode().strip()
    py_ldflags = subprocess.check_output(('python3-config', '--ldflags')).decode().strip()
    print(mk_cxx_build('$b_native/outpost_savegame.so', 'shlib',
        '$b_native/outpost_savegame_objs',
        ('$src/util/savegame_py/%s' % f
            for f in os.listdir(os.path.join(i.src_dir, 'util/savegame_py'))
            if f.endswith('.c')),
        cflags=py_includes,
        ldflags=py_ldflags,
        ))

    print('\n# Data processing')
    print(mk_font_build('$src/assets/misc/NeoSans.png', '$b_data/font'))
    print(mk_day_night_build('$src/assets/misc/day_night_pixels.png', '$b_data/day_night.json'))
    print(mk_server_json_build('$b_data/server.json'))

    print(mk_data_processing())
    print(mk_pack())

    print('\n# Javascript compilation')
    print(mk_js_opt_rules(i))
    print(mk_compile_modules(i, '$src/client/js/main.js', '$b_www/outpost.js'))
    print(mk_minify_asm('$b_asmjs/asmlibs.js'))
