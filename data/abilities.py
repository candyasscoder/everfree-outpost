from ..core.builder import *
from ..core.images import loader

from .lib.items import *


def init():
    tiles = loader('tiles')

    mk_item('ability/remove_hat', 'Remove Hat', tiles('equip_hat_icon_remove.png'))
    mk_item('ability/light', 'Light', tiles('candle32.png'))
