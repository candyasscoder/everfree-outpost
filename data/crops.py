from outpost_data.core.consts import *
from outpost_data.core.builder2 import *
from outpost_data.core.image2 import loader
from outpost_data.core import structure
from outpost_data.outpost.lib import models

def init():
    load = loader()
    struct_sheet = load('structures/crops.png', unit=TILE_SIZE)
    icon_sheet = load('icons/crops.png', unit=TILE_SIZE)

    def mk_crop(basename, display_name, index, size=(1, 1, 1), count=5):
        sx, sy, sz = size

        s = STRUCTURE.prefixed(basename) \
                .model(models.solid(*size)) \
                .shape(structure.solid(*size)) \
                .layer(1)
        for i in range(count):
            s.new(str(i)) \
                    .image(struct_sheet.extract((i, 2 * index), size=(sx, sy + sz)))

        ITEM.new(basename) \
                .display_name(display_name) \
                .icon(icon_sheet.extract((index, 0)))

    mk_crop('tomato', 'Tomato', 0)
    mk_crop('potato', 'Potato', 1)
    mk_crop('carrot', 'Carrot', 2)
    mk_crop('artichoke', 'Artichoke', 3)
    mk_crop('pepper', 'Pepper', 4)
    mk_crop('cucumber', 'Cucumber', 5)
    mk_crop('corn', 'Corn', 6, size=(1, 1, 2))
