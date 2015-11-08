from ..core.builder import *
from ..core.images import loader
from ..core.structure import Shape
from ..core.util import extract

from .lib.items import *


def init():
    gervais = loader('icons/gervais_roguelike')

    mk_item('pick', 'Pickaxe', gervais('AngbandTk_pick.png')) \
            .recipe('anvil', {'wood': 10, 'stone': 10}, count=5)
    mk_item('axe', 'Axe', gervais('AngbandTk_axe.png')) \
            .recipe('anvil', {'wood': 10, 'stone': 10})
    mk_item('mallet', 'Mallet', gervais('AngbandTk_mallet.png')) \
            .recipe('anvil', {'wood': 20})
    mk_item('shovel', 'Shovel', gervais('AngbandTk_shovel.png')) \
            .recipe('anvil', {'wood': 10, 'stone': 10})
