from outpost_data.core import util
from outpost_data.core.consts import *


class ItemDef(object):
    def __init__(self, name, ui_name, image):
        self.name = name
        self.ui_name = ui_name
        self.image = image

        self.id = None


def build_sheet(items):
    # No deduplication, since item ID corresponds directly to its position in
    # the sheet.
    num_pages, offsets = util.pack_boxes_uniform(SHEET_SIZE, len(items))
    assert num_pages == 1, 'too many item images to fit on one sheet'
    sheet, = util.build_sheets((i.image for i in items), offsets, 1, SHEET_SIZE, TILE_SIZE)
    return sheet


def build_client_json(items):
    def convert(i):
        return {
                'name': i.name,
                'ui_name': i.ui_name,
                }
    return list(convert(i) for i in items)

def build_server_json(items):
    def convert(i):
        return {
                'name': i.name,
                }
    return list(convert(i) for i in items)
