import argparse
import os
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


def mk_bindings(**kwargs):
    return '\n'.join('%s = %s' % (k, v) for k, v in sorted(kwargs.items()))

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
        )

    emit_bindings(
        rust_home=i.rust_home,
        bitflags_home=i.bitflags_home,

        rustc=i.rustc,
        python3='python3',
        cc='gcc',
        cxx='g++',
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
    rules = []
    def add_rule(*args, **kwargs):
        rules.append(mk_rule(*args, **kwargs))

    add_rule('rustc_native_bin',
        command=join(cmd_base, '--crate-type=bin', cond(i.debug, '', '-C lto')),
        description='RUSTC (native) $crate_name',
        depfile='$b_native/$crate_name.d',
        )

    add_rule('rustc_native_lib',
        command=join(cmd_base, '--crate-type=lib'),
        description='RUSTC (native) $crate_name',
        depfile='$b_native/$crate_name.d',
        )

    add_rule('c_obj',
        command=join('$cc -c $in -o $out -std=c99 -MMD -MF $out.d $picflag $cflags',
            cond(i.debug, '-ggdb', '-O3')),
        depfile='$out.d',
        description='CXX $out')

    add_rule('cxx_obj',
        command=join('$cxx -c $in -o $out -std=c++14 -MMD -MF $out.d $picflag $cxxflags',
            cond(i.debug, '-ggdb', '-O3')),
        depfile='$out.d',
        description='CXX $out')

    add_rule('link_bin',
        command=join('$cxx $in -o $out $ldflags $libs'),
        description='LD $out')

    add_rule('link_shlib',
        command=join('$cxx -shared $in -o $out $ldflags $libs'),
        description='LD $out')

    return '\n'.join(rules)

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

    rules = []
    def add_rule(*args, **kwargs):
        rules.append(mk_rule(*args, **kwargs))

    add_rule('asm_compile_rlib',
        command=join(compile_base, '--emit=link,dep-info', '--crate-type=rlib'),
        depfile='$b_asmjs/$crate_name.d',
        description='RUSTC (asmjs) $crate_name')

    add_rule('asm_compile_ir',
        # Like opt-level=3 above, lto is mandatory to prevent emscripten-fastcomp errors.
        command=join(compile_base, '--emit=llvm-ir,dep-info', '--crate-type=staticlib', '-C lto'),
        depfile='$b_asmjs/$crate_name.d',
        description='RUSTC (asmjs IR) $crate_name')

    add_rule('asm_clean_ir',
        command=join("sed <$in >$out",
            r"-e 's/\<dereferenceable([0-9]*)//g'",
            r"-e '/^!/s/\(.\)!/\1metadata !/g'",
            r"-e '/^!/s/distinct //g'",
            ),
        description='ASMJS CLEAN $out')

    add_rule('asm_assemble_bc',
        command='$em_llvm_as $in -o $out',
        description='ASMJS AS $out')

    add_rule('asm_optimize_bc',
        command=join('$em_opt $in',
            '-load=$em_pass_remove_overflow_checks',
            '-load=$em_pass_remove_assume',
            '-strip-debug',
            '-internalize -internalize-public-api-list="$$(cat $exports_file)"',
            '-remove-overflow-checks',
            '-remove-assume',
            '-globaldce',
            '-pnacl-abi-simplify-preopt -pnacl-abi-simplify-postopt',
            '-enable-emscripten-cxx-exceptions',
            '-o $out'),
        description='ASMJS OPT $out')

    add_rule('asm_convert_exports',
        command=r"tr '\n' ',' <$in >$out",
        description='ASMJS CONVERT_EXPORTS $out')

    add_rule('asm_generate_js',
        command=join('$em_llc $in',
            '-march=js -filetype=asm',
            '-emscripten-assertions=1',
            '-emscripten-no-aliasing-function-pointers',
            '-emscripten-max-setjmps=20',
            '-O3',
            '-o $out'),
        description='ASMJS LLC $out')

    add_rule('asm_add_function_tables',
        command='$python3 $src/util/asmjs_function_tables.py <$in >$out',
        description='ASMJS TABLES $out')

    add_rule('asm_insert_functions',
        command='awk -f $src/util/asmjs_insert_functions.awk <$in >$out',
        description='ASMJS AWK $out')

    return '\n'.join(rules)

def mk_asmjs_rlib(crate_name, deps, src_file=None):
    src_file = src_file or '$src/%s/lib.rs' % crate_name

    return mk_build(
            '$b_asmjs/lib%s.rlib' % crate_name, 
            'asm_compile_rlib',
            src_file,
            ('$b_asmjs/lib%s.rlib' % d for d in deps),
            crate_name=crate_name)

def mk_asmjs_asmlibs(name, rust_src, rust_deps, exports_file, template_file):
    f = lambda ext: '$b_asmjs/%s.%s' % (name, ext)

    builds = []
    def add_build(*args, **kwargs):
        builds.append(mk_build(*args, **kwargs))

    add_build(f('ll'), 'asm_compile_ir', rust_src,
            ('$b_asmjs/lib%s.rlib' % d for d in rust_deps),
            crate_name=name)
    add_build(f('clean.ll'), 'asm_clean_ir', f('ll'))
    add_build(f('bc'), 'asm_assemble_bc', f('clean.ll'))
    add_build(f('exports.txt'), 'asm_convert_exports', exports_file)
    add_build(f('opt.bc'), 'asm_optimize_bc', f('bc'),
            (f('exports.txt'),),
            exports_file=f('exports.txt'))
    add_build(f('0.js'), 'asm_generate_js', f('opt.bc'))
    add_build(f('1.js'), 'asm_add_function_tables', f('0.js'),
            ('$src/util/asmjs_function_tables.py',))
    add_build(f('js'), 'asm_insert_functions', template_file,
            (f('1.js'), '$src/util/asmjs_insert_functions.awk'))

    return '\n'.join(builds)


def mk_bitflags_fix(src_in, src_out):
    rule = mk_rule('fix_bitflags_src',
            command=
                "$\n  echo '#![feature(no_std)]' >$out && "
                "$\n  echo '#![no_std]' >>$out && "
                "$\n  cat $in >>$out",
            description='FIX bitflags.rs')

    build = mk_build(src_out, 'fix_bitflags_src', src_in)

    return rule + '\n' + build


if __name__ == '__main__':
    parser = build_parser()
    args = parser.parse_args(sys.argv[1:])
    i = Info(args)

    emit_header(i)

    print('\n# Native Rust compilation')
    print(mk_native_rules(i))

    print('\n# Asm.js Rust compilation')
    print(mk_asmjs_rules(i))

    print('\n# Native Rust compilation')
    print(mk_rustc_build('physics', 'lib', ()))
    print(mk_rustc_build('backend', 'bin', ('physics',), '$src/server/main.rs'))

    print('\n# Asm.js Rust compilation')
    print(mk_asmjs_rlib('core', (), '$rust_home/src/libcore/lib.rs'))
    print(mk_bitflags_fix('$bitflags_home/src/lib.rs', '$b_asmjs/bitflags.rs'))
    print(mk_asmjs_rlib('bitflags', (), '$b_asmjs/bitflags.rs'))
    print(mk_asmjs_rlib('asmrt', ('core',)))
    print(mk_asmjs_rlib('physics', ('core', 'bitflags', 'asmrt')))
    print(mk_asmjs_rlib('graphics', ('core', 'asmrt', 'physics')))
    print(mk_asmjs_asmlibs('asmlibs',
        '$src/client/asmlibs.rs', ('core', 'asmrt', 'physics', 'graphics'),
        '$src/client/asmlibs_exports.txt', '$src/client/asmlibs.tmpl.js'))

    print('\n# C/C++ compilation')
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
