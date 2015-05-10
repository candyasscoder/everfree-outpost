from PIL import Image

from outpost_data.builder import *
from outpost_data.consts import *

def init(asset_path):
    mk_tile('empty', Image.new('RGBA', (TILE_SIZE, TILE_SIZE)))
    mk_block('empty', 'empty', {})
    mk_item('none', 'Nothing', Image.new('RGBA', (TILE_SIZE, TILE_SIZE)))
