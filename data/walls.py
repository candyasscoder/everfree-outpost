from outpost_data.core.builder2 import STRUCTURE, ITEM, RECIPE
from outpost_data.core.consts import *
from outpost_data.core.image2 import loader
from outpost_data.core.structure import Shape, solid

from outpost_data.outpost.lib import models

OPEN_DOOR_SHAPE = Shape(3, 1, 2, [
        'solid', 'floor', 'solid',
        'solid', 'empty', 'solid',
        ])

CLOSED_DOOR_SHAPE = Shape(3, 1, 2, [
        'solid', 'solid', 'solid',
        'solid', 'solid', 'solid',
        ])

def do_wall_parts(basename, image, door_image=None, extra_parts=()):
    parts = (
            'corner/nw',
            'edge/horiz',
            'corner/ne',
            'corner/sw',
            '_/edge/horiz/copy',
            'corner/se',
            'edge/vert',
            'tee/e',
            'tee/w',
            'tee/n',
            'tee/s',
            'cross',
            # Doors are handled separately.
        )

    s = STRUCTURE.prefixed(basename) \
            .shape(solid(1, 1, 2)) \
            .layer(1)

    for j, part_name in enumerate(parts):
        if part_name.startswith('_'):
            continue
        s.new(part_name) \
                .part(models.WALL2[part_name], image.extract((j, 0), size=(1, 3)))

    for j, part_name in enumerate(extra_parts):
        x = len(parts) + (3 if door_image is not None else 0) + j
        s.new(part_name) \
                .part(models.WALL2['edge/horiz'], image.extract((x, 0), size=(1, 3)))

    if door_image is not None:
        doorway_img = image.extract((len(parts), 0), size=(3, 3))
        door_anim = door_image.sheet_to_anim((3, 3), door_image.size[0] // 3 * 4, oneshot=True)

        s.new('door/open').shape(OPEN_DOOR_SHAPE) \
                .part(models.WALL2['door'], door_anim.get_frame(-1)) \
                .part(models.WALL2['door'], doorway_img)
        s.new('door/closed').shape(CLOSED_DOOR_SHAPE) \
                .part(models.WALL2['door'], door_anim.get_frame(0)) \
                .part(models.WALL2['door'], doorway_img)

        s.new('door/opening').shape(CLOSED_DOOR_SHAPE) \
                .part(models.WALL2['door'], door_anim) \
                .part(models.WALL2['door'], doorway_img)
        s.new('door/closing').shape(CLOSED_DOOR_SHAPE) \
                .part(models.WALL2['door'], door_anim.reversed()) \
                .part(models.WALL2['door'], doorway_img)

    return s

def wall_items_recipes(wall, desc, material):
    wall_name = '%s Wall' % desc
    door_name = '%s Door' % desc

    item = ITEM.from_structure(wall['edge/horiz']) \
            .display_name(wall_name)
    recipe = RECIPE.from_item(item) \
            .input(material, 5) \
            .station('anvil')

    item = ITEM.from_structure(wall['door/closed']) \
            .display_name(door_name)
    recipe = RECIPE.from_item(item) \
            .input(material, 15) \
            .station('anvil')

def init():
    structures = loader('structures', unit=TILE_SIZE)

    wall = do_wall_parts('interior_wall', structures('interior-wall.png'),
            door_image=structures('door.png'))
    wall_items_recipes(wall, 'Interior', 'wood')

    wall = do_wall_parts('brick_wall', structures('brick-wall.png'),
            door_image=structures('door.png'))
    wall_items_recipes(wall, 'Brick', 'stone')

    wall = do_wall_parts('wood_wall', structures('wood_wall.png'),
            door_image=structures('door-rough.png'),
            extra_parts=('window/v0',))
    wall_items_recipes(wall, 'Wooden', 'wood')

    wall = do_wall_parts('stone_wall', structures('stone-wall.png'),
            door_image=structures('door.png'),
            extra_parts=('window/v0', 'window/v1'))
    wall_items_recipes(wall, 'Stone', 'stone')

    wall = do_wall_parts('ruined_wall', structures('ruined-wall.png'),
            door_image=structures('door.png'),
            extra_parts=('window/v0', 'window/v1'))
    wall_items_recipes(wall, 'Ruined', 'stone')

    wall = do_wall_parts('cottage_wall', structures('cottage-wall.png'),
            door_image=structures('door.png'),
            extra_parts=['window/v%d' % i for i in range(2)] +
                ['variant/v%d' % i for i in range(3)])
    wall_items_recipes(wall, 'Cottage', 'wood')
