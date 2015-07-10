from PIL import Image

from ..core.builder import *
from ..core.consts import *

def init():
    mk_tile('empty', Image.new('RGBA', (TILE_SIZE, TILE_SIZE)))
    mk_block('empty', 'empty', {})
    mk_item('none', 'Nothing', Image.new('RGBA', (TILE_SIZE, TILE_SIZE)))
