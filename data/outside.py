from outpost_data.builder import *
import outpost_data.images as I
from outpost_data.util import loader

def init(asset_path):
    tiles = loader(asset_path, 'tiles')
    structures = loader(asset_path, 'structures')
