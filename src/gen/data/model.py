from outpost_data.core.consts import *


class ModelDef(object):
    def __init__(self, name, verts):
        self.name = name
        self.verts = verts
        self.length = len(verts)

        # Set by build_array
        self.offset = None

def assign_offsets(models):
    models.sort(key=lambda m: m.name)
    idx = 0
    for m in models:
        m.offset = idx
        idx += m.length

    return dict((m.name, (m.offset, m.length)) for m in models)

def build_client_json(models):
    # Flatten out all coordinates of all vertices into one long list.
    arr = [x for m in models for v in m.verts for x in v]
    return arr
