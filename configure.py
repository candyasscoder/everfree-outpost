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

def maybe(s, x):
    if x is not None:
        return s % x
    else:
        return ''

def cond(x, a, b=''):
    if x:
        return a
    else:
        return b

def join(*args):
    return ' '.join(args)

def emit_header(i):
    print(mk_bindings(
        src=os.path.normpath(i.src_dir),
        # Note: (1) `build` is a ninja keyword; (2) `builddir` is a special variable that determines
        # where `.ninja_log` is stored.
        builddir=os.path.normpath(i.build_dir),
        dist=os.path.normpath(i.dist_dir),
    ))

    def b(*args):
        return os.path.normpath(os.path.join(i.build_dir, *args))

    # Make sure these come after the main definitions.
    print(mk_bindings(
        # Shorthand
        b_native=b('native'),
        ))

    print(mk_bindings(
        rustc=i.rustc,
        ))

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

    print(mk_rule('rustc_native_bin',
        command=join(cmd_base, '--crate-type=bin', cond(i.debug, '', '-C lto')),
        description='RUSTC (native) $crate_name',
        depfile='$b_native/$crate_name.d',
        ))

    print(mk_rule('rustc_native_lib',
        command=join(cmd_base, '--crate-type=lib'),
        description='RUSTC (native) $crate_name',
        depfile='$b_native/$crate_name.d',
        ))

    print(mk_build('$b_native/backend', 'rustc_native_bin', '$src/server/main.rs',
            ['$b_native/libphysics.rlib'],
            crate_name='backend',
            crate_type='bin'))

    print(mk_build('$b_native/libphysics.rlib', 'rustc_native_lib', '$src/physics/lib.rs',
            crate_name='physics'))

if __name__ == '__main__':
    parser = build_parser()
    args = parser.parse_args(sys.argv[1:])
    i = Info(args)

    emit_header(i)
    print('\n# Native Rust compilation')
    emit_native(i)
