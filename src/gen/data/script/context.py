"""Additional functions to import into the context of each compiled data
script."""

from outpost_data.core import depthmap, image2
from outpost_data.core.consts import *

def flat_depthmap(x, y):
    return image2.Image(img=depthmap.flat(x * TILE_SIZE, y * TILE_SIZE))
def solid_depthmap(x, y, z):
    return image2.Image(img=depthmap.solid(x * TILE_SIZE, y * TILE_SIZE, z * TILE_SIZE))
