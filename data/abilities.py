from outpost_data.builder import *
from outpost_data.util import loader

from lib.items import *


def init(asset_path):
    tiles = loader(asset_path, 'tiles')

    mk_item('ability/remove_hat', 'Remove Hat', tiles('equip_hat_icon_remove.png'))
    mk_item('ability/light', 'Light', tiles('candle32.png'))
