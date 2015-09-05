from ...core.builder import *
from ...core.consts import *
from ...core import depthmap
from ...core.structure import Shape, StaticAnimDef, floor, solid
from ...core.util import chop_terrain, chop_image, stack


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

    if x == 0 and y == 0 and w == image.size[0] and h == image.size[1]:
        struct_img = image
    else:
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

def mk_door_anim(basename, doorway_img, doorway_depth, door_img):
    open_door_shape_arr = [
            'solid', 'floor', 'solid',
            'solid', 'empty', 'solid',
            ]
    open_door_shape = Shape(3, 1, 2, open_door_shape_arr)

    closed_door_shape_arr = [
            'solid', 'solid', 'solid',
            'solid', 'solid', 'solid',
            ]
    closed_door_shape = Shape(3, 1, 2, closed_door_shape_arr)

    # Frames should be ordered Closed, Transitional..., Open.
    door_frames = chop_image(door_img, doorway_img.size)
    merged_frames = [stack(f, doorway_img) for _, f in sorted(door_frames.items())]

    b = structure_builder()
    if len(merged_frames) == 1:
        # Only have the "closed" frame.  Use empty doorway for "open".
        b.create(basename + '/closed', merged_frames[0], doorway_depth, closed_door_shape, 1)
        b.create(basename + '/open', doorway_img, doorway_depth, open_door_shape, 1)
        b.create(basename + '/closing', merged_frames[0], doorway_depth, closed_door_shape, 1)
        b.create(basename + '/opening', doorway_img, doorway_depth, closed_door_shape, 1)
    else:
        b.create(basename + '/closed', merged_frames[0], doorway_depth, closed_door_shape, 1)
        b.create(basename + '/open', merged_frames[-1], doorway_depth, open_door_shape, 1)

        rate = len(merged_frames) * 4
        open_anim = StaticAnimDef(merged_frames, rate, oneshot=True)
        close_anim = StaticAnimDef(list(reversed(merged_frames)), rate, oneshot=True)
        b.create(basename + '/closing', close_anim, doorway_depth, closed_door_shape, 1)
        b.create(basename + '/opening', open_anim, doorway_depth, closed_door_shape, 1)

    return b
