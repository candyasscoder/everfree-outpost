# Cornucopia mod

This mod adds a new structure, "cornucopia".  Any player that interacts with a
cornucopia will receive five vegetables of a randomly selected type.  Each
player can receive only one set of free vegetables.

The file `data.py` defines the new structure type and its appearance.  The file
`scripts.lua` defines the behavior of the cornucopia when players interact with
it.  See the comments in those files for more details.

The graphics for the new structure are located in
assets/structures/food_bag.png.  The assets/ directory also contains a file
SOURCES.yaml, which lists the authorship and licensing information for each
image in the assets directory.  This information is used to generate the game's
credits page.  Note that the build process will fail if any mod uses graphics
that do not have a matching SOURCES.yaml entry.
