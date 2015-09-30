import argparse
import importlib.abc
import importlib.machinery
import importlib.util
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

def add_script_import_hooks():
    import outpost_data.core.script
    import outpost_data.core.util

    def mk_init(script, module_name):
        # TODO: this code is a mess
        mod_name = outpost_data.core.util.extract_mod_name(module_name)
        def init():
            from outpost_data.core import builder, builder2, image2, consts, depthmap
            from outpost_data.core.script import Interpreter
            from outpost_data.core.consts import TILE_SIZE

            structure = builder2.StructureBuilder()
            item = builder2.ItemBuilder()
            recipe = builder2.RecipeBuilder()

            # Build context
            def flat_depthmap(x, y):
                return image2.Image(img=depthmap.flat(x * TILE_SIZE, y * TILE_SIZE))
            def solid_depthmap(x, y, z):
                return image2.Image(img=depthmap.solid(x * TILE_SIZE, y * TILE_SIZE, z * TILE_SIZE))
            ctx = {}
            ctx.update(consts.__dict__)
            ctx['flat_depthmap'] = flat_depthmap
            ctx['solid_depthmap'] = solid_depthmap

            # Run interpreter
            interp = Interpreter(dict(
                structure = structure,
                item = item,
                recipe = recipe,
                ), ctx, load_image=image2.loader(mod=mod_name))
            interp.run_script(script)

            def dump(b2, lst):
                for proto in b2._dct.values():
                    lst.append(proto.instantiate())
            dump(structure, builder.INSTANCE.structures)
            dump(item, builder.INSTANCE.items)
            dump(recipe, builder.INSTANCE.recipes)

        return init

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

            with open(self.origin) as f:
                script = outpost_data.core.script.parse_script(f.read(), self.origin)
            if script == None:
                raise ImportError('error parsing script %r' % self.origin)
            mod.init = mk_init(script, self.fullname)

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

    sys.meta_path.append(ScriptFinder)

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

    add_script_import_hooks()


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

    
    # Run `init()` for every mod.
    print('Loading mods:')
    for mod in mods:
        init_all(mod)


    # Generate output files 
    from outpost_data.core.gen import generate
    generate(ns.output_dir)


if __name__ == '__main__':
    main(sys.argv[1:])
