from .core import images
from .outpost.lib.structures import mk_solid_small

def init():
    # Define a new structure named "cornucopia" using `mk_solid_small` from the
    # base game's `structures` library (imported above).  This function defines
    # a 1-block structure with a 32x32 pixel image.  (The same function is used
    # to define the "anvil", "chest", and "teleporter" structures in the base
    # game.)
    #
    # The image for the structure is loaded from "structures/food_bag.png" in
    # the current mod's assets/ directory.
    mk_solid_small('cornucopia', images.load('structures/food_bag.png'))

    # Normally, a mod that defines a new structure will also define a craftable
    # item that allows players to place that structure.  (For example, the
    # "fence" item can be used to place the "fence" structure.)  In this case,
    # there is no item, because only superusers should be able to place the
    # cornucopia (with the /place command).
