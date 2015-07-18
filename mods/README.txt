Each subdirectory of this directory contains a single mod for Everfree Outpost.
A mod can add new items, abilities, blocks, structures, or equipment, and it
can also override properties of existing objects (defined by the base game or
by other mods).


# Mod structure

Each mod subdirectory must contain a "data script" and a "server-side script".

The data script runs during the build process and adds definitions for the new
items or other objects added by the mod.  These definitions are combined with
the definitions from the base game and other mods as part of the modpack build
process.  Simple mods can usually get by with a single "data.py" script in the
mod directory.  The script will be loaded and its "init()" function will be
called during the build process.  More complex mods can instead have a data/
directory containing several files, and the build process will automatically
load each one and run its init() function.

The server-side script is loaded as part of the server and controls the
behavior of the new objects.  For example, a mod that adds a new item must
include in its server-side script some code that runs when a player uses the
new item.  For simple mods, a single "scripts.lua" file may be sufficient.
More complex mods can have a scripts/ directory containing several files, and
each file will be loaded during server startup.

Most mods will also need an "assets" directory, which contains the graphics for
new objects introduced by the mod.  For example, a mod that adds a new item
should have an assets/ directory which contains the icon for the new item.

Finally, mods that alter existing objects may need an "assets_override"
directory.  Files in this directory can replace the graphics for mods loaded
earlier.  For example, "mod2" can override the graphics used in "mod1" by
placing new files in mods/mod2/assets_override/mod1/image.png.  Now when mod1
tries to load "image.png", it will get the version from mod2's assets_override
directory, instead of the original version from mods/mod1/assets/image.png.

# Examples

There are two example mods included here:
 - cornucopia: Adds a new structure that provides vegetables to any player that
   interacts with it.
 - explorer_hat: Replaces the default hat graphics with a new design.
