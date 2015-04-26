from PIL import Image

from outpost_data.consts import *


class TileDef(object):
    def __init__(self, name, image):
        self.name = name
        self.image = image

        self.id = None


def build_sheet(tiles):
    sheet_w, sheet_h = SHEET_SIZE
    num_rows = (len(tiles) + sheet_w) // sheet_w
    assert num_rows <= sheet_h
    sheet = Image.new('RGBA', (TILE_SIZE * sheet_w, TILE_SIZE * num_rows))

    for t in tiles:
        assert t.id < len(tiles)
        x = t.id % sheet_w
        y = t.id // sheet_w
        sheet.paste(t.image, (x * TILE_SIZE, y * TILE_SIZE))

    return sheet
