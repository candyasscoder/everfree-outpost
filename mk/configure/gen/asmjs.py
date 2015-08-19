import os

from configure.template import template
from configure.util import join, maybe


def rules(i):
    fastcomp = lambda p: os.path.join(i.emscripten_fastcomp_prefix, 'bin', p) \
            if i.emscripten_fastcomp_prefix is not None else p
    passes = lambda p: os.path.join(i.emscripten_passes_prefix, p) \
            if i.emscripten_passes_prefix is not None else p

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
        em_llvm_as = %{fastcomp('llvm-as')}
        em_opt = %{fastcomp('opt')}
        em_llc = %{fastcomp('llc')}

        em_pass_remove_overflow_checks = %{passes('RemoveOverflowChecks.so')}
        em_pass_remove_assume = %{passes('RemoveAssume.so')}


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
                -e '/^target triple/s/i686-unknown-linux-gnu/asmjs-unknown-emscripten/'
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
                -O3 $
                -o $out
            description = ASMJS $out

        rule asm_add_function_tables
            command = $python3 $root/mk/misc/asmjs_function_tables.py <$in >$out
            description = ASMJS $out

        rule asm_insert_functions
            command = $python3 $root/mk/misc/asmjs_insert_functions.py $in >$out
            description = ASMJS $out
    ''', **locals())

def rlib(crate_name, deps, src_file=None):
    src_file = src_file or '$root/%s/lib.rs' % crate_name

    return template('''
        build $b_asmjs/lib%{crate_name}.rlib: asm_compile_rlib %src_file $
            | %for d in deps% $b_asmjs/lib%{d}.rlib %end%
            crate_name = %crate_name
    ''', **locals())

def asmlibs(name, rust_src, rust_deps, exports_file, template_file):
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
            | $root/mk/misc/asmjs_function_tables.py
        build %base.js: asm_insert_functions %template_file %base.1.js $
            | $root/mk/misc/asmjs_insert_functions.py
    ''', base = '$b_asmjs/%s' % name, **locals())
