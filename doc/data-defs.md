# Data Definitions

Data definition files define structures, items, and recipes.  The objects
defined this way can then be used by players or scripts in-game.

Here are some example definitions:

    [structure anvil]
    image: "structures/anvil.png"
    model: `models.front(1, 1, 1)`
    shape: solid(1, 1, 1)
    layer: 1

    [item anvil]
    icon: "structures/anvil.png"
    display_name: "Anvil"

    [recipe anvil]
    display_name: "Anvil"
    station: anvil
    input: 10 wood
    input: 10 stone
    output: 1 anvil

This file consists of three sections.  Each section starts with a header that
gives the name and type of the object being defined, followed by fields that
set other properties of the object.

The first section in the example defines a structure named "anvil".  The
remaining fields set the appearance of the structure (`image`, `model`), the
shape of the structure for collision-detection purposes (`shape`), and the
layer number (`layer`).

The remainder of this document describes the allowed section and field types.

## Common

Names of objects are written without quotes and must not contain spaces,
hyphens, or other special characters aside from `_` and `/`.  Underscores are
used for separating multiple words (such as `cottage_wall`), and slashes are used
as hierarchical separators for indicating groups of related objects (such as
`cottage_wall/edge/vert`, `cottage_wall/corner/sw`, etc).

Display names are written with double quotes and may contain spaces.  These are
the names that will appear in the game's UI.  (Object names are used in scripts
but are never displayed to the player.)

Filenames are also double-quoted strings that refer to files (usually images).
Always use forward slashes (`/`) in filenames, even on Windows.  Filenames are
always relative to the `assets/` subdirectory (of either the current mod or the
game's base files).

Many fields are not required, or only one of a group of fields is required.
The data compiler will warn you if you accidentally leave out a required field.
The same field may also be set more than once, but for most fields only the
last setting in the section will be used.

## Structures

Valid fields for structures are:

 * `image` (filename): Names an image to be used as the appearance of the
   structure in the game world.  The image should normally be `32 * x` pixels
   wide and `32 * (y + z)` pixels high (where `(x, y, z)` is the size of the
   structure in grid cells), but for 1x1x1 structures it is also possible to
   use a 32x32 image that covers the front of the structure only.
 * `model` (model): Sets the 3D model used for the structure, which determines
   how lighting affects the structure and how it appears relative to nearby
   characters or overlapping structures.  Valid options are
   `\`models.bottom(x, y)\`` (for floor-like structures),
   `\`models.front(x, y, z)\`` (for 1x1x1 structures using a 32x32 image), and
   `\`models.solid(x, y, z)\`` (for other types of structures),
   where `(x, y, z)` is the size of the structure in grid cells.
 * `shape` (shape): Sets the shape of the structure for collision detection
   purposes.  This can be
   `empty(x, y, z)` (no collision),
   `floor(x, y, z)` (walkable floor), or
   `solid(x, y, z)` (completely blocked),
   where `(x, y, z)` is the size of the structure in grid cells.
 * `layer` (integer): Sets the layer number, which determines how the structure
   can overlap with other structures.  This should be
   0 for floor-like structures (such as house floors or roads),
   1 for most solid structures, and
   2 for structures that can be attached to a wall (such as bookshelves and cabinets).

## Items

Valid fields for items are:

 * `icon` (filename): Names an image to display as the appearance of the item
   in the game UI (inventory screens, etc).  This should be 16x16 for new
   items, though some older items use 32x32.
 * `display_name` (string): The item name that will be displayed in the game's
   UI.
 * `from_structure` (structure name): If this field is set, the icon will be
   automatically generated based on the appearance of the named structure.
   If set, this field must be the first field in the section.

## Recipes

Valid fields for recipes are:

 * `display_name` (string): The name of the recipe that will be displayed in
   the game's UI.
 * `station` (structure name): The name of the structure that is needed to
   craft this recipe.  Currently `anvil` is the only crafting structure in the
   base game, but mods may add others.
 * `input` (count + item name): Adds an input to the recipe.  This field may be
   specified more than once to add multiple inputs.
 * `output` (count + item name): Adds an output to the recipe.  This field may
   be specified more than once to add multiple outputs.
 * `from_item` (item name): If this field is set, the display name will be
   automatically set to the display name of the indicated item, and one copy of
   the item will be added as an output.
   If set, this field must be the first field in the section.

## Advanced

Data definitions are compiled to Python code for execution, and there are two
ways to embed additional Python code into the generated file.

First, writing `%%%` on a line by itself will begin a multi-line Python block,
which continues until the next `%%%`.  All code in the block will be added to
the module verbatim.  Python blocks cannot appear in the middle of the section
(they will break the section into two parts, causing an error).

Second, every field can accept a Python expression in backticks (`\``) in place
of an ordinary value.  The type of value produced by the expression must be
appropriate for the field: `str` for names and strings, pairs of `int` and
`str` for recipe inputs and outputs, and `Shape` / `Model` objects (from
`outpost_data.core.structure` for structure shapes and models.

It is possible to define multiple objects in a single section using backticked
expressions and the special `multi_names` field.  For example:

    [structure road]
    multi_names: `TERRAIN_PARTS2.keys()`

This will define a structure named `road/x` for each string `x` in
`TERRAIN_PARTS2.keys()`.  For the remaining fields, if the field is set to an
ordinary value, then each structure in the group will have the corresponding
field set the same way.  Alternatively, if the field is set to a `dict` (using
a backticked expression), then each object whose name matches a key will have
its field set to the corresponding value.
