from functools import lru_cache

from PIL import Image

def from_planemap(planemap):
    depthmap = Image.new('L', planemap.size)

    w, h = planemap.size

    #               *
    #               |
    #               |   horizontal
    #              /|
    #           * / *
    # equiv -+> |/  |
    #        v  |   |   vertical
    #          /|  /|   adjust: 
    #       *---* / *
    #        /  |/  |
    #       /   |   |   horizontal
    #      /   /|  /|
    #   *---*---*---*
    #
    # Vertical and horizontal segments with the same lower/right corner get
    # displayed approximately the same, so we don't distinguish them.  Adding 1
    # to `adjust` means the next pixel will be to the left of the current one;
    # adding nothing means the next will be above.

    for x in range(w):
        adjust = 0
        for y in reversed(range(h)):
            depthmap.putpixel((x, y), adjust)
            if planemap.getpixel((x, y))[0] == 0:
                adjust += 1

    return depthmap

@lru_cache()
def solid(w, h1, h2):
    depthmap = Image.new('L', (w, h1 + h2))

    adjust = 0
    for y in reversed(range(h1 + h2)):
        depthmap.paste(adjust, (0, y, w, y + 1))
        if y < h1:
            adjust += 1

    return depthmap

def flat(w, h):
    return solid(w, 0, h)
