from .block import BlockBuilder
from .structure import StructureBuilder
from .item import ItemBuilder
from .recipe import RecipeBuilder


INSTANCES = dict(
        block = BlockBuilder(),
        structure = StructureBuilder(),
        item = ItemBuilder(),
        recipe = RecipeBuilder(),
        )

BLOCK = INSTANCES['block']
STRUCTURE = INSTANCES['structure']
ITEM = INSTANCES['item']
RECIPE = INSTANCES['recipe']
