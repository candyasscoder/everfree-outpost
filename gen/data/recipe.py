from .consts import *
from .util import err


class RecipeDef(object):
    def __init__(self, name, ui_name, station, inputs, outputs):
        self.name = name
        self.ui_name = ui_name
        self.station_name = station
        self.input_names = tuple(inputs.items())
        self.output_names = tuple(outputs.items())

        self.id = None
        self.station_id = None
        self.input_ids = None
        self.output_ids = None

def resolve_item_ids(recipes, item_id_map):
    def go(recipe_name, name):
        id = item_id_map.get(name)
        if id is None:
            err('recipe %r: no such item: %r' % (recipe_name, name))
            return None
        return id

    for r in recipes:
        r.input_ids = tuple((go(r.name, k), v) for k,v in r.input_names)
        r.output_ids = tuple((go(r.name, k), v) for k,v in r.output_names)

def resolve_structure_ids(recipes, structure_id_map):
    for r in recipes:
        r.station_id = structure_id_map.get(r.station_name)
        if r.station_id is None:
            err('recipe %r: no such structure: %r' % (r.name, r.station_name))

def build_client_json(recipes):
    def convert(r):
        return {
                'ui_name': r.ui_name,
                'station': r.station_id,
                'inputs': r.input_ids,
                'outputs': r.output_ids,
                }
    return list(convert(r) for r in recipes)

def build_server_json(recipes):
    def convert(r):
        return {
                'name': r.name,
                'station': r.station_id,
                'inputs': r.input_ids,
                'outputs': r.output_ids,
                }
    return list(convert(r) for r in recipes)
