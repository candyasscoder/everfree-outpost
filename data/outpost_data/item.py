from outpost_data.consts import *


class ItemDef(object):
    def __init__(self, name, ui_name, image):
        self.name = name
        self.ui_name = ui_name
        self.image = image

        self.id = None

def build_client_json(items):
    def convert(i):
        return {
                'ui_name': i.ui_name,
                }
    return list(convert(i) for i in items)

def build_server_json(items):
    def convert(i):
        return {
                'name': i.name,
                }
    return list(convert(i) for i in items)
