# Loot Tables

Loot tables are used to customize certain aspects of world generation.  When
world generation needs to choose a random structure for a particular location
or random items to place in a chest, it uses a loot table to make the decision.
Mods can extend loot tables so that newly added structures or items will appear
in generated worlds.

There are two types of loot table.   "Choose" tables select one entry from a
list of possibilities.  "Multi" tables select multiple entries, based on an
independent chance of inclusion for each entry.

## Choose

Here is an example of a `choose_item` table:

    [choose_item cave/chest/small]
    (1) 80-120 stone
    (1) 80-120 wood
    (2) 15-20 crystal

This code defines a `choose_item` table named `cave/chest/small`.  This table
is used to set the contents of chests generated in caves.  It has three
possible outcomes: the chest may contain stone (in a stack of 80-120 items),
wood (80-120 items), or crystal (15-20 items).  It will never contain more than
one type of item, such as stone and crystal together.

The numbers in parentheses give the weight of each outcome.  The weight
controls how likely each outcome is relative to other outcomes.  Since the
weight for crystal (2) is double that for stone (1), the chest will contain
crystal twice as often as it contains stone.

## Multi

Here is an example of a `multi_item` table:

    [multi_item cave/chest]
    *cave/chest/small
    (2%) *cave/chest/large

This defines the `cave/chest` table, which is the main table consulted by world
generation when filling in chests in caves.  The `cave/chest/small` table above
is referenced by this table, using the `*name` syntax.  The outcome of the
`cave/chest` table always includes the result of choosing from the
`cave/chest/small` table, and 2% of the time, it also adds the result from the
`cave/chest/large` table.

The numbers in parentheses here give the chance of including each entry.  If no
chance is specified, it is treated as 100% (the entry is always included).
Unlike the weights in "choose" tables, these chances are independent.

## Structures

Both examples above have shown tables for selecting items.  Tables for
structure selection are similar, but only the `choose_structure` table type is
allowed.  There is no `multi_structure`, because in many cases it is not
possible to place two structures at the same location.  For the same reason,
there is no quantity field (the "80-120" in "80-120 stone") for structure
entries.

## Extension

Extensions are defined similar to ordinary tables, but they add entries to an
existing table instead of defining a new table.  Here is an example:

    [choose_structure_ext cave/floor/small]
    (1) ore_vein/copper

This is an extension for the `cave/floor/small` table (which is a
`choose_structure` table).  It adds an entry for the `ore_vein/copper`
structure, with weight 1.  This definition appears in the ore\_vein mod to
cause cave generation to include copper ore veins among the other generated
structures.
