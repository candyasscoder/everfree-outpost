from PIL import Image

from ...core.builder import *
from ...core.consts import *
from ...core.structure import StructureDef
from ...core.util import extract


def mk_structure_item(s, name, ui_name, base=None):
    if not isinstance(s, StructureDef):
        s = s.unwrap()

    if base is None:
        orig = s.get_image()
        w, h = orig.size
        side = max(w, h)
        img = Image.new('RGBA', (side, side))
        img.paste(orig, ((side - w) // 2, (side - h) // 2))
        img = img.resize((TILE_SIZE, TILE_SIZE), resample=Image.BILINEAR)
    else:
        img = extract(s.get_image(), base)

    return mk_item(name, ui_name, img)

