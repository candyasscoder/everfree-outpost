import os

from configure.template import template
from configure.util import join, maybe


def rules(i):
    return template('''
        rule scripts_stamp
            command = touch $out
            description = STAMP $out

        rule copy_mod_scripts
            command = $python3 $src/mk/misc/copy_mod_scripts.py $
                --mod-name $mod_name $
                --input-dir $input_dir $
                --output-dir $output_dir $
                --stamp $out
            description = COPY $output_dir/$mod_name ($out)
            depfile = $out.d

        rule copy_script
            command = cp -f $in $out
            description = COPY $out

        rule gen_script_loader
            command = $python3 $src/mk/misc/gen_script_loader.py $
                --script-dir $script_dir $
                --mods $mods $
                --output $out
            description = GEN $out
    ''', **locals())

def copy_mod_scripts(mods):
    builds = []
    def add_build(*args, **kwargs):
        builds.append(template(*args, **kwargs))

    for mod in mods:
        if mod != 'outpost':
            input_dir = os.path.join('$src', 'mods', mod, 'scripts')
        else:
            input_dir = os.path.join('$src', 'scripts', 'outpost')

        add_build('''
            build $b_scripts/stamp/%mod: copy_mod_scripts $
                    | $src/mk/misc/copy_mod_scripts.py
                mod_name = %mod
                input_dir = %input_dir
                output_dir = $b_scripts/gen
        ''', **locals())

    add_build('''
        build $b_scripts/stamp/core: copy_mod_scripts $
                | $src/mk/misc/copy_mod_scripts.py
            mod_name = core
            input_dir = $src/scripts/core
            output_dir = $b_scripts/gen

        build $b_scripts/gen/loader.lua: gen_script_loader | $
                %for mod in mods
                $b_scripts/stamp/%mod $
                %end
                $src/mk/misc/gen_script_loader.py
            script_dir = $b_scripts/gen
            mods = $mods

        build $b_scripts/gen/bootstrap.lua: copy_script $src/scripts/bootstrap.lua

        build $b_scripts/stamp/_all: scripts_stamp | $
            %for mod in mods
            $b_scripts/stamp/%mod $
            %end
            $b_scripts/gen/bootstrap.lua $
            $b_scripts/gen/loader.lua $
            $b_scripts/stamp/core
    ''', **locals())

    return '\n\n'.join(builds)
