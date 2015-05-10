from outpost_data.builder import *
from outpost_data.consts import *
from outpost_data import depthmap
from outpost_data.structure import Shape, floor, solid
from outpost_data.util import chop_terrain


def mk_terrain_structures(basename, image):
    structs = structure_builder()
    depth = depthmap.flat(TILE_SIZE, TILE_SIZE)
    shape = floor(1, 1, 1)

    for k, tile in chop_terrain(image).items():
        name = basename + '/' + k
        structs.create(name, tile, depth, shape, 0)

    return structs

def mk_solid_structure(name, image, size, base=(0, 0), display_size=None,
        plane_image=None, layer=1):
    base_x, base_y = base
    x = base_x * TILE_SIZE
    y = base_y * TILE_SIZE

    size_x, size_y, size_z = size
    if display_size is not None:
        display_size_x, display_size_y = display_size
        w = display_size_x * TILE_SIZE
        h = display_size_y * TILE_SIZE
    else:
        w = size_x * TILE_SIZE
        h = (size_y + size_z) * TILE_SIZE

    struct_img = image.crop((x, y, x + w, y + h))
    if plane_image is None:
        depth = depthmap.solid(size_x * TILE_SIZE, size_y * TILE_SIZE, size_z * TILE_SIZE)
        # Cut a w*h sized section from the bottom.
        depth_height = depth.size[1]
        depth = depth.crop((0, depth_height - h, w, depth_height))
    else:
        depth = depthmap.from_planemap(plane_image.crop((x, y, x + w, y + h)))

    return mk_structure(name, struct_img, depth, solid(*size), layer)

def mk_solid_small(name, image, **kwargs):
    """Make a small, solid structure: a solid structure with size 1x1x1, but
    only a 1x1 tile (for the front, nothing on the top)."""
    return mk_solid_structure(name, image, (1, 1, 1), display_size=(1, 1), **kwargs)
