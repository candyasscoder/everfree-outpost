# Ore Vein mod

This mod adds copper ore veins and several related items to the game.
Specifically, it adds:

 * Structure `ore_vein/copper`: A copper ore vein that spawns in caves.
 * Item `ore/copper`: Copper ore, collected by mining a vein with a pickaxe.
 * Item `bar/copper`: Copper bars, made from ore.
 * Recipe `bar/copper`: Crafting recipe for turning copper ore into bars.
 * Recipe `pick/copper`: Crafting recipe for making a "higher-durability pick"
   (actually just a large stack of regular picks, since durability is not yet
   implemented) using copper bars.

## Files

This section describes the purpose of every file in the mod.

 * `README.md`: You are here.

 * `data/ore_vein.od`: Defines all the new objects that this mod adds to the
   game.  That is, this file defines the structure, the two inventory items,
   and the two recipes listed above.

 * `data/add_ore_vein.loot`: Makes ore veins spawn in caves, by modifying the
   loot table used for placing structures in cave interiors.

 * `scripts.lua`: Contains the server-side scripts that govern the behavior of
   the new objects.  Specifically, it contains some code that makes using a
   pickaxe on an ore vein produce copper ore (and destroy the vein).

 * `assets/icons/ore-copper.png`: The inventory icon for copper ore.  This and
   other graphical elements are described in more detail where they are
   referenced in `data/ore_vein.od`.

 * `assets/icons/bar-copper.png`: The inventory icon for copper bars.

 * `assets/structures/ore-vein.png`: The graphics used for displaying ore veins
   in the world.

 * `assets/SOURCES.yaml`: Gives authorship and attribution information for all
   images in the `assets/` directory.

All these files (except the PNGs) are text files, and contain comments giving
more details on the individual pieces.


## Exercise

Here is a starter project to try out modding for yourself.

**Goal**: Add a furnace that the player can use for crafting copper bars from
ore.

This requires making the following changes:

 1. Add a furnace structure that can be placed in the world.  This requires
    artwork and a new data definition.
    
    For the artwork, you can either draw it yourself or just use the oven from
    `assets/tiles/lpc-base-tiles/kitchen.png`.  Add the image to the ore vein
    mod's `assets/structures` directory.  Also make sure you update the mod's
    `assets/SOURCES.yaml` with the proper authorship information for the new
    artwork.

    For the data definition, add a new `structure` section to
    `data/ore_vein.od`.  You can use the existing `ore_vein/copper` or the
    structure definitions from the main `data/test2.od` file as references.
    You may also need to refer to `doc/data-defs.md`.  Make sure the image size
    and the data definition size match up (the image should be 32x32 or 32x64
    depending on what model you set in the structure definition).

    At this point, you should be able to build the mod, run a server, and
    `/place furnace` (or whatever internal name you chose) to see how the new
    structure looks in-game.  (Note you must first give yourself superuser
    privileges on the server to use `/place`.)

 2. Add a new item that can be used to place the furnace.  This requires a new
    data definition and some scripting.

    For the data definition, add a new `item` section, similar to the anvil
    `item` section in `data/test2.od`.  If you use the `from_structure` field
    for the item definition, the item icon will be generated automatically from
    the structure's image, so you don't need to add any additional artwork.

    For the scripting, you need to update the ore vein mod's `scripts.lua` so
    that using the new "furnace" item places a "furnace" structure in the
    world.  Refer to `scripts/anvil.lua` for an example of how to set this up
    (particularly the `action.use_item.anvil` handler).  If you make any
    mistakes here, you should be able to see the relevant error messages in the
    server log.

    You should also add a bit more scripting so that using a pickaxe on the
    furnace will destroy it and return the furnace item to the player.  Refer
    to `anvil.lua` again, this time the `tools.handler.pick.anvil` function,
    which is called when a player uses a pickaxe on an anvil.

    At this point, you should be able to test the mod by using `/give furnace`
    (or whatever internal name you chose for the item) and then using the item
    to place a furnace in the world.  Don't forget that you need to rebuild the
    mod and restart the server to make changes to the scripts or data
    definitions take effect.

 3. Add a new recipe so that players can craft furnaces.  This requires only a
    new data definition.
    
    For this, add a new `recipe` section to `ore_vein.od`.  You can use the
    existing `bar/copper` recipe or the `anvil` recipe from `data/test2.od` as
    a reference.  Choose something reasonable for the recipe inputs.

    At this point, you should be able to see and craft the new "Furnace" recipe
    listed in the anvil crafting screen.

 4. Make the furnace usable for crafting.  This requires only a script change.

    For this, update the ore vein mod's `scripts.lua` with a function that
    calls `open_crafting` on the client when the player uses the furnace.  It
    should look like the `action.use.anvil` handler from
    `scripts/outpost/anvil.lua`.

    At this point, you should be able to press A on an anvil and get the
    crafting screen, though it will have no recipes listed at the moment.

 5. Update the copper bar recipe to require a furnace instead of an anvil.
    This requires only a small change to `ore_vein.od`: just switch the
    `station` field of the `bar/copper` recipe to refer to the furnace
    structure type instead of the anvil.

    At this point, you should be done!  You should be able to craft a furnace
    item, place the furnace into the world, and then use the furnace to craft
    copper bars from ore.

