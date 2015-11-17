# Game Objects

The game state contains several types of objects, described below.

## Terrain

The game world is a three-dimensional grid, made up of *blocks* and
*structures*.  Each cell of the grid contains one block, which may be empty
space, grass, or part of a cliff wall.  Blocks are "mostly permanent", in that
they change only rarely.  Structures are build on top of blocks and are aligned
to the same grid.  They may cover more than one cell, they change more often,
and they can have scripts to react to player interaction.  Trees, anvils, and
house walls are all examples of structures.  Both structures and blocks are
stationary and may block player movement (they have collision detection
shapes).

The grid is divided into equally-spaced, non-overlapping regions called
*chunks*.  This way the server does not need to store the entire map in memory
at once - it can load only the chunks that are close to players.  Most parts of
the code operate on grid coordinates, ignoring the division of the grid into
chunks, but it is still important to know that distant objects may not be
loaded.

The game world actually contains multiple independent grids, called *planes*.
For example, each dungeon is a separate plane, independent of the forest and
other dungeons.  A player could theoretically walk from any point in the forest
to any other point, but they can cross from the forest plane to a dungeon plane
only through a scripted teleport.  Since coordinates in different planes are
independent, coordinates such as 0,0,0 do not uniquely specify a position
(every plane has something different at 0,0,0).  Nearly all code that uses
coordinates also must track the ID of the relevant plane.

## Players and characters

An *entity* is an object that can move through the world.  Currently the only
entities are player characters.  Entity positions are not aligned to the grid,
so they can move in increments of one pixel instead of 32 (the size of a grid
cell).  Entities can be blocked by structures or terrain, but they cannot block
other entities.

Player-character entities are attached to a *client*, which represents a
logged-in player.  When the player sends input (key presses), the server looks
up the player's client object, finds the entity controlled by that client
(called the client's "pawn"), and updates the entity's movement speed or
direction based on the input.  When the player logs out, the client and entity
objects are both removed from the world and written to the save file.

An entity may have one or more *inventories* attached to it, each consisting of
a list of items.  Player characters have an inventory for items held by the
character.  Structures may also have attached inventories.  For example,
a container structure such as a chest or barrel will have an inventory to track
its contents.  Players can move objects between inventories, or use crafting to
add new items to their inventory.  Characters' abilities are also tracked using
a special inventory that the player can't manipulate directly.
