import argparse
import importlib.abc
import importlib.machinery
import importlib.util
import json
import os
import sys
import time

def build_parser():
    args = argparse.ArgumentParser()

    args.add_argument('--mods',
            help='list of mods to include in the generated files')
    args.add_argument('--src-dir',
            help='path to the outpost source directory')
    args.add_argument('--output-dir',
            help='directory to contain the generated files')

    return args


class TimeIt(object):
    def __init__(self, msg):
        self.msg = msg
        self.start = None

    def __enter__(self):
        sys.stdout.write(self.msg)
        sys.stdout.flush()
        self.start = time.time()

    def __exit__(self, exc_type, exc_value, traceback):
        dur = time.time() - self.start
        sys.stdout.write('  (%d ms)\n' % (dur * 1000))


# A bunch of weird import machinery used to support mods.
#
# We put everything into a virtual package called `outpost_data`, structured
# like this:
#
#    outpost_data           - no real location
#     - core                - src/gen/data/
#     - outpost             - data/
#     - my_mod              - mods/my_mod/data/
#     - ...
#
# This setup allows mods to access functions provided by their dependencies,
# just by importing from `..other_mod.lib`.

def attach_to_package(name):
    """Attach the named module to its parent package.  For example, if `name`
    is "foo.bar", then do `sys.modules["foo"].bar = sys.modules["foo.bar"]`."""
    pkg_name, _, base_name = name.rpartition('.')
    if pkg_name not in sys.modules:
        return
    setattr(sys.modules[pkg_name], base_name, sys.modules[name])

def load_from_path(name, path):
    """Import a module (`name`) from a source file (`path`)."""
    if name not in sys.modules:
        if path.endswith('.od'):
            loader = ScriptLoader(name, path)
            loader.load_module(name)
        else:
            loader = importlib.machinery.SourceFileLoader(name, path)
            loader.load_module()
        attach_to_package(name)
    return sys.modules[name]

class FakePackage(object):
    """Object that emulates a package with an empty `__init__.py`.  Useful for
    virtual packages that don't have an `__init__.py` anywhere on disk."""
    def __init__(self, name, path, all=None):
        self.__name__ = name
        self.__package__ = name
        self.__path__ = [path]
        if all is None:
            all = tuple(f[:-3] for f in os.listdir(path)
                    if f.endswith('.py') or f.endswith('.od'))
        self.__all__ = all

def load_fake_package(name, path, **kwargs):
    """Import a fake package (`name`) with a given source directory."""
    if name not in sys.modules:
        sys.modules[name] = FakePackage(name, path, **kwargs)
        attach_to_package(name)
    return sys.modules[name]

class ScriptLoader(importlib.abc.Loader):
    def __init__(self, fullname, origin):
        self.fullname = fullname
        self.origin = origin

    @importlib.util.module_for_loader
    def load_module(self, mod):
        package, _, basename = self.fullname.rpartition('.')

        mod.__name__ = self.fullname
        mod.__file__ = self.origin
        mod.__package__ = package
        mod.__loader__ = self

        try:
            import outpost_data.core.script
            compiler = outpost_data.core.script.Compiler(self.origin)

            with open(self.origin) as f:
                script = outpost_data.core.script.parse_script(f.read(), self.origin)
            if script == None:
                raise ValueError('error parsing script %r' % self.origin)
            code = compiler.compile_module(script)
            exec(code, mod.__dict__)
        except Exception as e:
            import traceback
            traceback.print_exc()
            raise

class ScriptFinder(importlib.abc.MetaPathFinder, importlib.abc.Loader):
    def find_module(fullname, path):
        if path is None:
            return None

        package, _, basename = fullname.rpartition('.')
        filename = basename + '.od'
        for d in path:
            file_path = os.path.join(d, filename)
            if os.path.exists(file_path):
                return ScriptLoader(fullname, file_path)
        return None


DEPENDENCIES = set()

def load_mod(name, path):
    """Load the data definition module/package for a given game mod."""
    module_name = 'outpost_data.' + name

    init_py = os.path.join(path, '__init__.py')
    if os.path.isfile(init_py):
        return load_from_path(module_name, init_py)
    elif os.path.isfile(path + '.py'):
        # NB: Depends on the nonexistence of foo/__init__.py, but we can't
        # express that easily.  (Adding foo/ to deps would force a rebuild
        # every time if foo/ does not exist.)
        return load_from_path(module_name, path + '.py')
    elif os.path.isfile(path + '.od'):
        return load_from_path(module_name, path + '.od')
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
    """Call the `init()` function of a module, or do so recursively for all
    submodules within a package (by consulting `__all__`)."""
    if hasattr(mod, 'init'):
        with TimeIt('  %s' % mod.__name__.partition('.')[2]):
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

    # Register `ScriptFinder` for loading .od modules.
    sys.meta_path.append(ScriptFinder)


    from outpost_data.core import files

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
            files.register_mod(mod_name, asset_dir, override_dir, deps)

    from outpost_data.core import util
    if util.SAW_ERROR:
        sys.exit(1)


    # Image cache handling
    from outpost_data.core import image_cache
    with TimeIt('Loading cache...'):
        try:
            with open(os.path.join(ns.output_dir, 'image_cache.pickle'), 'rb') as f:
                image_cache.load_cache(f)
        except Exception:
            pass
        sys.stdout.write(' %d images' % len(image_cache.IMAGE_CACHE))

    # Run `init()` for every mod.
    print('Processing mods:')
    for mod in mods:
        init_all(mod)


    # Generate output files 
    from outpost_data.core.gen import generate
    generate(ns.output_dir)

    with TimeIt('Saving cache...'):
        with open(os.path.join(ns.output_dir, 'image_cache.pickle'), 'wb') as f:
            image_cache.dump_cache(f)
            sys.stdout.write(' %d images' % len(image_cache.NEW_IMAGE_CACHE))
    

if __name__ == '__main__':
    main(sys.argv[1:])
