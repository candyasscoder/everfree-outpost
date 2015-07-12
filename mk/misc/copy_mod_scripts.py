import argparse
import os
import shutil
import sys


def build_parser():
    args = argparse.ArgumentParser()

    args.add_argument('--mod-name',
            help='name of the mod')
    args.add_argument('--input-dir',
            help='directory containing the mod scripts')
    args.add_argument('--output-dir',
            help='directory to contain the copied files')
    args.add_argument('--stamp',
            help='path to the stamp file')

    return args

def main(args):
    ns = build_parser().parse_args(args)

    os.makedirs(ns.output_dir, exist_ok=True)

    # The mod can provide either `foo/scripts/` or `foo/scripts.lua`.
    file_in = ns.input_dir + '.lua'
    file_out = os.path.join(ns.output_dir, ns.mod_name + '.lua')

    dir_in = ns.input_dir
    dir_out = os.path.join(ns.output_dir, ns.mod_name)

    if os.path.exists(file_out):
        os.remove(file_out)
    if os.path.exists(dir_out):
        shutil.rmtree(dir_out)

    deps = [os.path.dirname(ns.input_dir)]
    if os.path.exists(dir_in):
        shutil.copytree(dir_in, dir_out)

        for (dirpath, dirnames, filenames) in os.walk(dir_in):
            deps.append(dirpath)
            deps.extend(os.path.join(dirpath, fn) for fn in filenames)
    else:
        shutil.copy(file_in, file_out)
        deps.append(file_in)


    with open(ns.stamp, 'w') as f:
        pass

    with open(ns.stamp + '.d', 'w') as f:
        f.write('%s: \\\n' % ns.stamp)
        for path in deps:
            f.write('    %s \\\n' % path)

if __name__ == '__main__':
    main(sys.argv[1:])
