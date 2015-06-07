from outpost_data.builder import *
import outpost_data.images as I
from outpost_data import depthmap
from outpost_data.structure import Shape
from outpost_data.util import loader, extract

from lib.structures import *
from lib.terrain import *


def mk_crop(basename, sheet, base_y, count=5, size=(1, 1, 1)):
    for i in range(count):
        mk_solid_structure('%s/%d' % (basename, i), sheet, size, base=(i, base_y))


def init(asset_path):
    tiles = loader(asset_path, 'tiles')
    structures = loader(asset_path, 'structures')

    img = structures('crops.png')

    mk_crop('tomato', img, 0)
    mk_crop('potato', img, 2)
    mk_crop('carrot', img, 4)
    mk_crop('artichoke', img, 6)
    mk_crop('pepper', img, 8)
    mk_crop('cucumber', img, 10)
    mk_crop('corn', img, 12, size=(1, 1, 2))

    img = tiles('daneeklu_farming_tilesets/plants.png')
    mk_item('tomato', 'Tomato', extract(img, (0, 11)))
    mk_item('potato', 'Potato', extract(img, (1, 11)))
    mk_item('carrot', 'Carrot', extract(img, (2, 11)))
    mk_item('artichoke', 'Artichoke', extract(img, (3, 11)))
    mk_item('pepper', 'Pepper', extract(img, (4, 11)))
    mk_item('cucumber', 'Cucumber', extract(img, (5, 11)))
    mk_item('corn', 'Corn', extract(img, (6, 11)))
