from ..core.builder2 import *
from ..core import image2
from ..core import depthmap
from ..core.structure import Shape, solid as solid_shape, floor as floor_shape

from ..core.consts import *
from ..core.script import parse_script, Interpreter

TERRAIN_PARTS2 = dict((n, (x, y))
        for y, row in enumerate(TERRAIN_PARTS)
        for x, n in enumerate(row))

def flat_depthmap(x, y):
    return image2.Image(img=depthmap.flat(x * TILE_SIZE, y * TILE_SIZE))

def solid_depthmap(x, y, z):
    return image2.Image(img=depthmap.solid(x * TILE_SIZE, y * TILE_SIZE, z * TILE_SIZE))

def init():
    from pprint import pprint
    img = image2.loader()

    structure = StructureBuilder()
    item = ItemBuilder()
    recipe = RecipeBuilder()

    interp = Interpreter(dict(
        structure = structure,
        item = item,
        recipe = recipe,
        ), globals())

    interp.run_script(parse_script('''

        [structure road]
        multi_names: `TERRAIN_PARTS2.keys()`
        image: `load("structures/road.png").chop(TERRAIN_PARTS2, unit=TILE_SIZE)`
        depthmap: `flat_depthmap(1, 1)`
        shape: floor(1, 1, 1)
        layer: 0

        [item road]
        from_structure: road/center/v0
        display_name: "Road"

        [recipe road]
        from_item: road
        station: anvil
        input: 5 stone


        [structure anvil]
        image: "structures/anvil.png"
        depthmap: `solid_depthmap(1, 0, 1)`
        shape: solid(1, 1, 1)
        layer: 1

        [item anvil]
        from_structure: anvil
        display_name: "Anvil"

        [recipe anvil]
        from_item: anvil
        station: anvil
        input: 10 wood
        input: 10 stone


        [structure chest]
        image: "structures/chest.png"
        depthmap: `solid_depthmap(1, 0, 1)`
        shape: solid(1, 1, 1)
        layer: 1

        [item chest]
        from_structure: chest
        display_name: "Chest"

        [recipe chest]
        from_item: chest
        station: anvil
        input: 20 wood


        [structure barrel]
        image: "structures/barrel.png"
        depthmap: `solid_depthmap(1, 1, 1)`
        shape: solid(1, 1, 1)
        layer: 1

        [item barrel]
        from_structure: barrel
        display_name: "Barrel"

        [recipe barrel]
        from_item: barrel
        station: anvil
        input: 20 wood

        '''))


    from ..core import builder
    def dump(b2, lst):
        for proto in b2._dct.values():
            lst.append(proto.instantiate())
    dump(structure, builder.INSTANCE.structures)
    dump(item, builder.INSTANCE.items)
    dump(recipe, builder.INSTANCE.recipes)
