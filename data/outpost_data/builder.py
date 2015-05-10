from outpost_data import structure, tile, block, item


class Objects(object):
    def __init__(self, owner):
        self.owner = owner
        self.x = {}

    def _add(self, obj):
        self.x[obj.name] = obj

    def filter(self, pred):
        result = type(self)(self.owner)
        result.x = dict((k, v) for (k, v) in self.x.items() if pred(v))
        return result

    def _foreach(self, f):
        for v in self.x.values():
            f(v)


class Tiles(Objects):
    def create(self, name, image):
        self._add(tile.TileDef(name, image))
        return self

class Blocks(Objects):
    def create(self, name, shape, tiles):
        tile_names = {}
        for k,v in tiles.items():
            if v is None:
                continue
            elif isinstance(v, str):
                tile_names[k] = v
            else:
                tile_names[k] = self.owner.gen_tile('%s/%s' % (name, k), v)

        self._add(block.BlockDef(name, shape, tile_names))
        return self

    def light(self, color, radius):
        self._foreach(lambda s: s.set_light(color, radius))
        return self

class Structures(Objects):
    def create(self, name, image, depthmap, shape, layer):
        self._add(structure.StructureDef(name, image, depthmap, shape, layer))
        return self

    def light(self, pos, color, radius):
        self._foreach(lambda s: s.set_light(pos, color, radius))
        return self

class Items(Objects):
    def create(self, name, ui_name, image):
        self._add(item.ItemDef(name, ui_name, image))
        return self

    def recipe(self, station, inputs, count=1):
        def go(i):
            self.owner.mk_recipe(r.name, r.ui_name, station, inputs, {r.name: count})
        self._foreach(go)
        return self

class Recipes(Objects):
    def create(self, name, ui_name, station, inputs, outputs):
        self._add(structure.StructureDef(name, ui_name, station, inputs, outputs))
        return self


class Builder(object):
    def __init__(self):
        self.structures = []
        self.tiles = []
        self.blocks = []
        self.items = []
        self.recipes = []

        self.gen_tile_cache = {}

    def gen_tile(self, base_name, img):
        data = tuple(img.getdata())
        h = hash(data)

        for cache_img, name in self.gen_tile_cache.get(h, ()):
            if tuple(cache_img.getdata()) == data:
                return name

        # Image is not in the cache.
        name = '_auto/%s' % base_name
        self.mk_tile(name, img)
        self.gen_tile_cache.setdefault(h, []).append((img, name))
        return name


    def tile_builder(self):
        return Tiles(self)

    def mk_tile(self, *args, **kwargs):
        return self.tile_builder().create(*args, **kwargs)


    def block_builder(self):
        return Blocks(self)

    def mk_block(self, *args, **kwargs):
        return self.block_builder().create(*args, **kwargs)


    def structure_builder(self):
        return Structures(self)

    def mk_structure(self, *args, **kwargs):
        return self.structure_builder().create(*args, **kwargs)


    def item_builder(self):
        return Items(self)

    def mk_item(self, *args, **kwargs):
        return self.item_builder().create(*args, **kwargs)


    def recipe_builder(self):
        return recipes(self)

    def mk_recipe(self, *args, **kwargs):
        return self.recipe_builder().create(*args, **kwargs)


INSTANCE = Builder()
mk_tile = INSTANCE.mk_tile
mk_block = INSTANCE.mk_block
mk_structure = INSTANCE.mk_structure
mk_item = INSTANCE.mk_item
mk_recipe = INSTANCE.mk_recipe

tile_builder = INSTANCE.tile_builder
block_builder = INSTANCE.block_builder
structure_builder = INSTANCE.structure_builder
item_builder = INSTANCE.item_builder
recipe_builder = INSTANCE.recipe_builder
