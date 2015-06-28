import argparse
import os
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

def emit_native(i):
    cmd_base = join('$rustc $in',
            '--out-dir $b_native',
            '--emit link,dep-info',
            '-L $b_native',
            maybe('-L %s', i.rust_extra_libdir),
            cond(i.debug, '', '-C opt-level=3'),
            maybe('--extern log=%s/liblog.rlib', i.rust_extra_libdir),
            maybe('--extern rand=%s/librand.rlib', i.rust_extra_libdir),
            )

    emit_rule('rustc_native_bin',
        command=join(cmd_base, '--crate-type=bin', cond(i.debug, '', '-C lto')),
        description='RUSTC (native) $crate_name',
        depfile='$b_native/$crate_name.d',
        )

    emit_rule('rustc_native_lib',
        command=join(cmd_base, '--crate-type=lib'),
        description='RUSTC (native) $crate_name',
        depfile='$b_native/$crate_name.d',
        )

    emit_build('$b_native/backend', 'rustc_native_bin', '$src/server/main.rs',
            ['$b_native/libphysics.rlib'],
            crate_name='backend')

    emit_build('$b_native/libphysics.rlib', 'rustc_native_lib', '$src/physics/lib.rs',
            crate_name='physics')

def emit_asmjs(i):
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

    emit_rule('asm_compile_rlib',
        command=join(compile_base, '--emit=link,dep-info', '--crate-type=rlib'),
        description='RUSTC (asmjs) $crate_name',
        depfile='$b_asmjs/$crate_name.d',
        )

    emit_rule('asm_compile_ir',
        # Like opt-level=3 above, lto is mandatory to prevent emscripten-fastcomp errors.
        command=join(compile_base, '--emit=llvm-ir,dep-info', '--crate-type=staticlib', '-C lto'),
        description='RUSTC (asmjs IR) $crate_name',
        depfile='$b_asmjs/$crate_name.d',
        )

    emit_rule('fix_bitflags_src',
        command=
            "$\n  echo '#![feature(no_std)]' >$out && "
            "$\n  echo '#![no_std]' >>$out && "
            "$\n  cat $in >>$out",
        description='PATCH bitflags.rs',
            )

    def rlib(crate_name, src_file, deps):
        emit_build('$b_asmjs/lib%s.rlib' % crate_name, 'asm_compile_rlib',
            src_file,
            ('$b_asmjs/lib%s.rlib' % x for x in deps),
            crate_name=crate_name)

    rlib('core', '$rust_home/src/libcore/lib.rs', ())
    rlib('bitflags', '$b_asmjs/bitflags.rs', ())
    emit_build('$b_asmjs/bitflags.rs', 'fix_bitflags_src',
        '$bitflags_home/src/lib.rs')

    local_rlib = lambda cn, deps: rlib(cn, '$src/%s/lib.rs' % cn, deps)

    local_rlib('asmrt', ('core',))
    local_rlib('physics', ('core', 'bitflags', 'asmrt'))
    local_rlib('graphics', ('core', 'asmrt', 'physics'))


    emit_rule('asm_clean_ir',
        command=join("sed <$in >$out",
            r"-e 's/\<dereferenceable([0-9]*)//g'",
            r"-e '/^!/s/\(.\)!/\1metadata !/g'",
            r"-e '/^!/s/distinct //g'",
            ),
        description='ASMJS CLEAN $out',
        )

    emit_rule('asm_assemble_bc',
        command='$em_llvm_as $in -o $out',
        description='ASMJS AS $out',
        )

    emit_rule('asm_optimize_bc',
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
        description='ASMJS OPT $out',
    )

    emit_rule('asm_convert_exports',
        command=r"tr '\n' ',' <$in >$out",
        description='ASMJS CONVERT_EXPORTS $out')

    emit_rule('asm_generate_js',
        command=join('$em_llc $in',
            '-march=js -filetype=asm',
            '-emscripten-assertions=1',
            '-emscripten-no-aliasing-function-pointers',
            '-emscripten-max-setjmps=20',
            '-O3',
            '-o $out'),
        description='ASMJS LLC $out')

    emit_rule('asm_add_function_tables',
        command='$python3 $src/util/asmjs_function_tables.py <$in >$out',
        description='ASMJS TABLES $out')

    emit_rule('asm_insert_functions',
        command='awk -f $src/util/asmjs_insert_functions.awk <$in >$out',
        description='ASMJS AWK $out')

    emit_build('$b_asmjs/asmlibs.ll', 'asm_compile_ir', '$src/client/asmlibs.rs',
        ('$b_asmjs/lib%s.rlib' % x for x in ('core', 'asmrt', 'physics', 'graphics')),
        crate_name='asmlibs')
    emit_build('$b_asmjs/asmlibs.clean.ll', 'asm_clean_ir', '$b_asmjs/asmlibs.ll')
    emit_build('$b_asmjs/asmlibs.bc', 'asm_assemble_bc', '$b_asmjs/asmlibs.clean.ll')
    emit_build('$b_asmjs/asmlibs.exports.txt', 'asm_convert_exports',
        '$src/client/asmlibs_exports.txt')
    emit_build('$b_asmjs/asmlibs.opt.bc', 'asm_optimize_bc', '$b_asmjs/asmlibs.bc',
            ('$b_asmjs/asmlibs.exports.txt',),
            exports_file='$b_asmjs/asmlibs.exports.txt')
    emit_build('$b_asmjs/asmlibs.0.js', 'asm_generate_js', '$b_asmjs/asmlibs.opt.bc')
    emit_build('$b_asmjs/asmlibs.1.js', 'asm_add_function_tables', '$b_asmjs/asmlibs.0.js',
            ('$src/util/asmjs_function_tables.py',))
    emit_build('$b_asmjs/asmlibs.js', 'asm_insert_functions', '$src/client/asmlibs.tmpl.js',
            ('$b_asmjs/asmlibs.1.js', '$src/util/asmjs_insert_functions.awk'))

if __name__ == '__main__':
    parser = build_parser()
    args = parser.parse_args(sys.argv[1:])
    i = Info(args)

    emit_header(i)
    print('\n# Native Rust compilation')
    emit_native(i)
    print('\n# Asm.js Rust compilation')
    emit_asmjs(i)
