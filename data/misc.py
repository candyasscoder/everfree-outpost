from ..core.builder import *
from ..core.images import loader
from ..core.structure import Shape
from ..core.util import extract

from .lib.items import *
from .lib.structures import *


def init():
    tiles = loader('tiles')
    structures = loader('structures')

    teleporter = mk_solid_small('teleporter', structures('crystal-formation.png')) \
            .light((16, 16, 16), (48, 48, 96), 50)
    mk_structure_item(teleporter, 'teleporter', 'Teleporter') \
            .recipe('anvil', {'crystal': 50})

    ward = mk_solid_structure('ward', structures('crystal-ward.png'), (1, 1, 1)) \
            .light((16, 16, 32), (48, 48, 96), 50)
    mk_item('ward', 'Ward', extract(structures('crystal-ward.png'), (1, 1))) \
            .recipe('anvil', {'wood': 10, 'crystal': 1})

    mk_solid_small('dungeon_entrance', structures('crystal-formation-red.png')) \
            .light((16, 16, 16), (96, 48, 48), 50)
    mk_solid_small('dungeon_exit', structures('crystal-formation-red.png')) \
            .light((16, 16, 16), (96, 48, 48), 50)
    mk_solid_small('script_trigger', structures('crystal-formation-green.png')) \
            .light((16, 16, 16), (48, 96, 48), 50)


    mk_item('crystal', 'Crystal', extract(structures('crystal-ward.png'), (1, 0)))

    mk_item('hat', 'Hat', tiles('equip_hat_icon.png'))


    sign = mk_solid_structure('sign', structures('sign.png'), (1, 1, 1))
    mk_structure_item(sign, 'sign', 'Sign') \
            .recipe('anvil', {'wood': 5})


    image = structures('pillar.png')
    pillar = mk_solid_structure('pillar/wood', image, (1, 1, 2), base=(0, 0))
    mk_structure_item(pillar, 'wood_pillar', 'Wooden Pillar') \
            .recipe('anvil', {'wood': 5})
    pillar = mk_solid_structure('pillar/stone', image, (1, 1, 2), base=(1, 0))
    mk_structure_item(pillar, 'stone_pillar', 'Stone Pillar') \
            .recipe('anvil', {'stone': 5})
