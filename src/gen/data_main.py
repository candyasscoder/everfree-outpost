import argparse
import importlib.machinery
import json
import os
import sys

def build_parser():
    args = argparse.ArgumentParser()

    args.add_argument('--mods',
            help='list of mods to include in the generated files')
    args.add_argument('--src-dir',
            help='path to the outpost source directory')
    args.add_argument('--output-dir',
            help='directory to contain the generated files')

    return args


SOURCE_FILES = set()

def attach_to_package(name):
    pkg_name, _, base_name = name.rpartition('.')
    if pkg_name not in sys.modules:
        return
    setattr(sys.modules[pkg_name], base_name, sys.modules[name])

def load_from_path(name, path):
    if name not in sys.modules:
        loader = importlib.machinery.SourceFileLoader(name, path)
        loader.load_module()
        attach_to_package(name)
    return sys.modules[name]

class FakePackage(object):
    def __init__(self, name, path, all=None):
        self.__name__ = name
        self.__package__ = name
        self.__path__ = [path]
        if all is None:
            all = tuple(f[:-3] for f in os.listdir(path) if f.endswith('.py'))
        self.__all__ = all

def load_fake_package(name, path, **kwargs):
    if name not in sys.modules:
        sys.modules[name] = FakePackage(name, path, **kwargs)
        attach_to_package(name)
    return sys.modules[name]


DEPENDENCIES = set()

def load_mod(name, path):
    module_name = 'outpost_data.' + name

    init_py = os.path.join(path, '__init__.py')
    if os.path.isfile(init_py):
        return load_from_path(module_name, init_py)
    elif os.path.isfile(path + '.py'):
        # NB: Depends on the nonexistence of foo/__init__.py, but we can't
        # express that easily.  (Adding foo/ to deps would force a rebuild
        # every time if foo/ does not exist.)
        return load_from_path(module_name, path + '.py')
    elif os.path.isdir(path):
        # NB: Depends on the nonexistence of foo.py, but we can't express that
        # easily.
        DEPENDENCIES.add(path)
        return load_fake_package(module_name, path)
    else:
        return load_fake_package(module_name, path, all=())

def get_dependencies():
    deps = list(DEPENDENCIES)
    mods = (v for k,v in sys.modules.items() if k.startswith('outpost_data.'))
    for k, v in ((k, v) for k, v in sys.modules.items() if k.startswith('outpost_data.')):
        f = getattr(v, '__file__', None)
        if f is not None:
            deps.append(f)
    return deps


def init_all(mod):
    if hasattr(mod, 'init'):
        print('  %s' % mod.__name__.partition('.')[2])
        getattr(mod, 'init')()
    elif hasattr(mod, '__all__'):
        for name in sorted(getattr(mod, '__all__')):
            submod = importlib.import_module(mod.__name__ + '.' + name)
            init_all(submod)
    else:
        raise TypeError('module %s (%s) contains neither `init` nor `__all__`' %
                (mod.__name__, getattr(mod, '__file__', '<no path>')))

def main(args):
    ns = build_parser().parse_args(args)

    # Set up `outpost_data.core` package.
    load_fake_package('outpost_data', ns.src_dir)
    load_from_path('outpost_data.core',
            os.path.join(ns.src_dir, 'src', 'gen', 'data', '__init__.py'))

    sys.modules['outpost_data.core.loader'] = sys.modules[__name__]
    attach_to_package('outpost_data.core.loader')


    from outpost_data.core import images

    # Load mods and set up asset search path.
    mods = []
    seen_outpost = False

    if ns.mods is not None:
        for mod_name in ns.mods.split(','):
            if mod_name != 'outpost':
                mod_dir = os.path.join(ns.src_dir, 'mods', mod_name)
                data_dir = os.path.join(mod_dir, 'data')
                asset_dir = os.path.join(mod_dir, 'assets')
                override_dir = os.path.join(mod_dir, 'asset_overrides')
                deps = ('outpost',) if seen_outpost else ()
            else:
                data_dir = os.path.join(ns.src_dir, 'data')
                asset_dir = os.path.join(ns.src_dir, 'assets')
                override_dir = None
                deps = ()
                seen_outpost = True

            mods.append(load_mod(mod_name, data_dir))
            images.register_mod(mod_name, asset_dir, override_dir, deps)

    from outpost_data.core import util
    if util.SAW_ERROR:
        sys.exit(1)

    
    # Run `init()` for every mod.
    print('Loading mods:')
    for mod in mods:
        init_all(mod)


    # Generate output files 
    from outpost_data.core.gen import generate
    generate(ns.output_dir)


if __name__ == '__main__':
    main(sys.argv[1:])
