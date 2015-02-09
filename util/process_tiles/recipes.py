from itertools import count

from process_tiles import tree
from process_tiles.util import combine_prefix

from process_tiles.blocks import assign_ids

def parse_raw(yaml):
    t = tree.expand_tree(yaml, '/.')
    tree.tag_leaf_dicts(t, {'inputs', 'outputs'})
    tree.apply_defaults(t)
    recipes = tree.flatten_tree(t)

    assign_ids(recipes)
    for k,v in recipes.items():
        v['name'] = k

    return recipes

def build_client_json(recipe_arr, items_by_name, objects_by_name):
    def go(recipe):
        if recipe is None:
            return None

        name = recipe['name']
        ui_name = recipe['ui_name']
        station = objects_by_name[recipe['station']]['id']
        inputs = [(objects_by_name[k]['id'], v) for k,v in recipe['inputs'].items()]
        outputs = [(objects_by_name[k]['id'], v) for k,v in recipe['outputs'].items()]

        return {
                'name': name,
                'ui_name': ui_name,
                'station': station,
                'inputs': inputs,
                'outputs': outputs,
                }

    return [go(i) for i in recipe_arr]

def build_server_json(recipe_arr, items_by_name, objects_by_name):
    def go(recipe):
        if recipe is None:
            return None

        name = recipe['name']
        station = objects_by_name[recipe['station']]['id']
        inputs = [(objects_by_name[k]['id'], v) for k,v in recipe['inputs'].items()]
        outputs = [(objects_by_name[k]['id'], v) for k,v in recipe['outputs'].items()]

        return {
                'name': name,
                'station': station,
                'inputs': inputs,
                'outputs': outputs,
                }

    return [go(i) for i in recipe_arr]
