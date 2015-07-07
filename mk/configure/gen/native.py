import os

from configure.template import template
from configure.util import cond, join, maybe, mk_build


def rules(i):
    rustc_base = join('$rustc $in',
            '--out-dir $b_native',
            '--emit link,dep-info',
            '-L $b_native',
            maybe('-L %s', i.rust_extra_libdir),
            i.rust_lib_externs,
            cond(i.debug, '', '-C opt-level=3'),
            )

    common_cflags = join(
            '-MMD -MF $out.d',
            '$picflag',
            cond(i.debug, '-ggdb', '-O3'),
            )

    return template('''
        rule rustc_native_bin
            command = %rustc_base --crate-type=bin  %if i.debug% -C lto %end%
            depfile = $b_native/$crate_name.d
            description = RUSTC $out

        rule rustc_native_lib
            command = %rustc_base --crate-type=lib
            depfile = $b_native/$crate_name.d
            description = RUSTC $out

        rule c_obj
            command = $cc -c $in -o $out -std=c99 %common_cflags $cflags $user_cflags
            depfile = $out.d
            description = CC $out

        rule cxx_obj
            command = $cxx -c $in -o $out -std=c++14 %common_cflags $cxxflags $user_cxxflags
            depfile = $out.d
            description = CXX $out

        rule link_bin
            command = $cxx $in -o $out $ldflags $user_ldflags $libs
            description = LD $out

        rule link_shlib
            command = $cxx -shared $in -o $out $ldflags $user_ldflags $libs
            description = LD $out
    ''', **locals())


def rust(crate_name, crate_type, deps, src_file=None):
    if crate_type == 'bin':
        output_name = '%s$_exe' % crate_name
        src_file = src_file or '$src/%s/main.rs' % crate_name
    elif crate_type == 'lib':
        output_name = 'lib%s.rlib' % crate_name
        src_file = src_file or '$src/%s/lib.rs' % crate_name

    return template('''
        build $b_native/%output_name: rustc_native_%{crate_type} %src_file $
            | %for d in deps% $b_native/lib%{d}.rlib %end%
            crate_name = %crate_name
    ''', **locals())

def cxx(out_name, out_type, src_files, **kwargs):
    builds = []
    def add_build(*args, **kwargs):
        builds.append(mk_build(*args, **kwargs))

    out_file = ('%s$_exe' if out_type == 'bin' else '%s$_so') % out_name
    out_path = '$b_native/%s' % out_file
    pic_flag = '' if out_type == 'bin' else '-fPIC'

    deps = []
    for f in src_files:
        obj_file = '$b_native/%s_objs/%s.o' % (out_name, os.path.basename(f))
        if f.endswith('.c'):
            build_type = 'c_obj'
        else:
            build_type = 'cxx_obj'
        add_build(obj_file, build_type, f, picflag=pic_flag, **kwargs)
        deps.append(obj_file)

    add_build(out_path, 'link_%s' % out_type, deps, **kwargs)

    return '\n'.join(builds)
