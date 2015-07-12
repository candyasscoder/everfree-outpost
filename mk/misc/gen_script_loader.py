import argparse
import os
import shutil
import sys


def build_parser():
    args = argparse.ArgumentParser()

    args.add_argument('--script-dir',
            help='directory containing the mod scripts')
    args.add_argument('--mods',
            help='list of mods to load')
    args.add_argument('--output',
            help='path to the file to generate')

    return args

def main(args):
    ns = build_parser().parse_args(args)

    ext_modules = []
    modules = []

    for mod in ns.mods.split(','):
        mod_base = os.path.join(ns.script_dir, mod)

        if os.path.isdir(mod_base):
            # Record all .lua files under foo/ext/ as extension modules.
            ext_dir = os.path.join(mod_base, 'ext')
            if os.path.isdir(ext_dir):
                for ext_file in os.listdir(ext_dir):
                    if not ext_file.endswith('.lua'):
                        continue
                    ext_name, _ = os.path.splitext(ext_file)
                    ext_modules.append('%s.ext.%s' % (mod, ext_name))

            # Record all .lua modules under foo/ as normal modules.
            for module_file in os.listdir(mod_base):
                if not module_file.endswith('.lua'):
                    continue
                module_name, _ = os.path.splitext(module_file)
                modules.append('%s.%s' % (mod, module_name))
        else:
            modules.append(mod)

    with open(ns.output, 'w') as f:
        for m in ext_modules:
            f.write('require("%s")\n' % m)
        for m in modules:
            f.write('require("%s")\n' % m)

if __name__ == '__main__':
    main(sys.argv[1:])
