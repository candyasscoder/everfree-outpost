from . import structure, tile, block, item, recipe, animation


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

    def merge(self, other):
        assert(type(self) is type(other))
        for k, v in other.x.items():
            self.x[k] = v

    def _foreach(self, f):
        for v in self.x.values():
            f(v)

    def __getitem__(self, k):
        return self.x[k]

    def unwrap(self):
        assert len(self.x) == 1
        return next(iter(self.x.values()))


class Tiles(Objects):
    def create(self, name, image):
        t = tile.TileDef(name, image)
        self._add(t)
        self.owner.tiles.append(t)
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

        b = block.BlockDef(name, shape, tile_names)
        self._add(b)
        self.owner.blocks.append(b)
        return self

    def light(self, color, radius):
        self._foreach(lambda s: s.set_light(color, radius))
        return self

class Structures(Objects):
    def create(self, name, image, depthmap, shape, layer):
        s = structure.StructureDef(name, image, depthmap, shape, layer)
        self._add(s)
        self.owner.structures.append(s)
        return self

    def light(self, pos, color, radius):
        self._foreach(lambda s: s.set_light(pos, color, radius))
        return self

class Items(Objects):
    def create(self, name, ui_name, image):
        i = item.ItemDef(name, ui_name, image)
        self._add(i)
        self.owner.items.append(i)
        return self

    def recipe(self, station, inputs, count=1):
        def go(i):
            self.owner.mk_recipe(i.name, i.ui_name, station, inputs, {i.name: count})
        self._foreach(go)
        return self

class Recipes(Objects):
    def create(self, name, ui_name, station, inputs, outputs):
        r = recipe.RecipeDef(name, ui_name, station, inputs, outputs)
        self._add(r)
        self.owner.recipes.append(r)
        return self

class AnimGroups(Objects):
    def create(self, name):
        g = animation.AnimGroupDef(name)
        self._add(g)
        self.owner.anim_groups.append(g)
        return self

    def add_anim(self, name, length, framerate):
        def go(g):
            g.add_anim(name, length, framerate)
        self._foreach(go)
        return self

    def add_anim_mirror(self, name, orig_name):
        def go(g):
            g.add_anim_mirror(name, orig_name)
        self._foreach(go)
        return self

    def finish(self):
        def go(g):
            g.finish()
            for a in g.anims.values():
                self.owner.animations.append(a)
        self._foreach(go)
        return self

class Builder(object):
    def __init__(self):
        self.structures = []
        self.tiles = []
        self.blocks = []
        self.items = []
        self.recipes = []
        self.anim_groups = []
        self.animations = []

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
        return Recipes(self)

    def mk_recipe(self, *args, **kwargs):
        return self.recipe_builder().create(*args, **kwargs)


    def anim_group_builder(self):
        return AnimGroups(self)

    def mk_anim_group(self, *args, **kwargs):
        return self.anim_group_builder().create(*args, **kwargs)


INSTANCE = Builder()
mk_tile = INSTANCE.mk_tile
mk_block = INSTANCE.mk_block
mk_structure = INSTANCE.mk_structure
mk_item = INSTANCE.mk_item
mk_recipe = INSTANCE.mk_recipe
mk_anim_group = INSTANCE.mk_anim_group

tile_builder = INSTANCE.tile_builder
block_builder = INSTANCE.block_builder
structure_builder = INSTANCE.structure_builder
item_builder = INSTANCE.item_builder
recipe_builder = INSTANCE.recipe_builder
anim_group_builder = INSTANCE.anim_group_builder
