from outpost_data.builder import *
import outpost_data.images as I
from outpost_data import depthmap
from outpost_data.structure import Shape
from outpost_data.util import loader, extract

from lib.items import *


def init(asset_path):
    tiles = loader(asset_path, 'tiles')
    gervais = loader(asset_path, 'tiles/gervais_roguelike')

    mk_item('pick', 'Pickaxe', gervais('AngbandTk_pick.png')) \
            .recipe('anvil', {'wood': 10, 'stone': 10})
    mk_item('axe', 'Axe', gervais('AngbandTk_axe.png')) \
            .recipe('anvil', {'wood': 10, 'stone': 10})
    mk_item('mallet', 'Mallet', gervais('AngbandTk_mallet.png')) \
            .recipe('anvil', {'wood': 20})
