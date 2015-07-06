import os

from configure.template import template
from configure.util import cond, join, maybe, mk_build


def rules(i):
    return template('''
        rule js_compile_modules
            command = $
                %if not i.debug
                $closure_compiler $
                    $$($python3 $src/mk/misc/collect_js_deps.py $in $out $depfile) $
                    --js_output_file=$out $
                    --language_in=ECMASCRIPT5_STRICT $
                    --compilation_level=ADVANCED_OPTIMIZATIONS $
                    --output_wrapper='(function(){%{}output%{}})();' $
                    --jscomp_error=undefinedNames $
                    --jscomp_error=undefinedVars $
                    --create_name_map_files $
                    --process_common_js_modules $
                    --common_js_entry_module=$entry_module $
                    --common_js_module_path_prefix=$module_dir $
                    --externs=$src/mk/misc/closure_externs.js
                %else
                $python3 $src/mk/misc/gen_js_loader.py $
                    $$($python3 $src/mk/misc/collect_js_deps.py $in $out $depfile) $
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

def compile(i, out_file, main_src):
    main_dir, basename_ext = os.path.split(main_src)
    module_name, _ = os.path.splitext(basename_ext)

    return template('''
        build %out_file: js_compile_modules %main_src $
            | $src/mk/misc/collect_js_deps.py $
              %if i.debug% $src/mk/misc/gen_js_loader.py %end%
            entry_module = %module_name
            module_dir = %main_dir
    ''', **locals())

def minify(out_file, js_src):
    if out_file is None:
        out_file = '$b_js/%s' % os.path.basename(js_src)

    return template('''
        build %out_file: js_minify_file %js_src
            filter = | sed -e '1s/{/{"use asm";/'
    ''', **locals())
