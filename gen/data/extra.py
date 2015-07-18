class ExtraDef(object):
    def __init__(self, name, func):
        self.name = name
        self.func = func

        self.value = None

    def resolve(self, builder, id_maps):
        self.value = self.func(builder, id_maps)

def resolve_all(extras, builder, id_maps):
    for e in extras:
        e.resolve(builder, id_maps)


# JSON output

def build_client_json(extras):
    return dict((e.name, e.value) for e in extras)
