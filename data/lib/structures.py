from ...core.builder import *
from ...core.consts import *
from ...core.structure import Shape, StaticAnimDef, floor, solid
from ...core.util import chop_terrain, chop_image, stack

from outpost_data.outpost.lib import models


def mk_terrain_structures(basename, image):
    structs = structure_builder()
    model = models.bottom(1, 1)
    shape = floor(1, 1, 1)

    for k, tile in chop_terrain(image).items():
        name = basename + '/' + k
        structs.create(name, tile, model, shape, 0)

    return structs

def mk_solid_structure(name, image, size, base=(0, 0), display_size=None,
        model=None, layer=1):
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
    if model is None:
        model = models.solid(size_x, size_y, size_z)

    return mk_structure(name, struct_img, model, solid(*size), layer)

def mk_solid_small(name, image, **kwargs):
    """Make a small, solid structure: a solid structure with size 1x1x1, but
    only a 1x1 tile (for the front, nothing on the top)."""
    return mk_solid_structure(name, image, (1, 1, 1), display_size=(1, 1), **kwargs)

def mk_door_anim(basename, doorway_img, doorway_model, door_img, framerate=0):
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
        b.create(basename + '/closed', merged_frames[0], doorway_model, closed_door_shape, 1)
        b.create(basename + '/open', doorway_img, doorway_model, open_door_shape, 1)
        b.create(basename + '/closing', merged_frames[0], doorway_model, closed_door_shape, 1)
        b.create(basename + '/opening', doorway_img, doorway_model, closed_door_shape, 1)
    else:
        b.create(basename + '/closed', merged_frames[0], doorway_model, closed_door_shape, 1)
        b.create(basename + '/open', merged_frames[-1], doorway_model, open_door_shape, 1)

        rate = framerate or (len(merged_frames) * 4)
        open_anim = StaticAnimDef(merged_frames, rate, oneshot=True)
        close_anim = StaticAnimDef(list(reversed(merged_frames)), rate, oneshot=True)
        b.create(basename + '/closing', close_anim, doorway_model, closed_door_shape, 1)
        b.create(basename + '/opening', open_anim, doorway_model, closed_door_shape, 1)

    return b
