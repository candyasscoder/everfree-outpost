from outpost_data import structure, tile, block


class Builder(object):
    def __init__(self):
        self.structures = []
        self.tiles = []
        self.blocks = []

        self.gen_tile_cache = {}

    def mk_tile(self, name, image):
        t = tile.TileDef(name, image)
        self.tiles.append(t)
        return t

    def mk_block(self, name, shape, tiles):
        tile_names = {}
        for k,v in tiles.items():
            if v is None:
                continue
            elif isinstance(v, str):
                tile_names[k] = v
            else:
                tile_names[k] = self.gen_tile('%s/%s' % (name, k), v)

        b = block.BlockDef(name, shape, tile_names)
        self.blocks.append(b)
        return b

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

    def mk_structure(self, name, image, depthmap, shape, layer):
        s = structure.StructureDef(name, image, depthmap, shape, layer)
        self.structures.append(s)
        return s


INSTANCE = Builder()
mk_tile = INSTANCE.mk_tile
mk_block = INSTANCE.mk_block
mk_structure = INSTANCE.mk_structure
