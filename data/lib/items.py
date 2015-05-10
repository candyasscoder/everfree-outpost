from PIL import Image

from outpost_data.builder import *
from outpost_data.consts import *
from outpost_data.structure import StructureDef
from outpost_data.util import extract


def mk_structure_item(s, name, ui_name, base=None):
    if not isinstance(s, StructureDef):
        s = s.unwrap()

    if base is None:
        w, h = s.image.size
        side = max(w, h)
        img = Image.new('RGBA', (side, side))
        img.paste(s.image, ((side - w) // 2, (side - h) // 2))
        img = img.resize((TILE_SIZE, TILE_SIZE), resample=Image.BILINEAR)
    else:
        img = extract(s.image, base)

    return mk_item(name, ui_name, img)

