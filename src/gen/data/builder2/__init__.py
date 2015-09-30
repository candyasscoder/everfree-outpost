from .structure import StructureBuilder
from .item import ItemBuilder
from .recipe import RecipeBuilder


INSTANCES = dict(
        structure = StructureBuilder(),
        item = ItemBuilder(),
        recipe = RecipeBuilder(),
        )

STRUCTURE = INSTANCES['structure']
ITEM = INSTANCES['item']
RECIPE = INSTANCES['recipe']
