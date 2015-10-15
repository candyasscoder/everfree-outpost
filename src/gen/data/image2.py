import PIL

from . import files
from . import util


def t2(x):
    if isinstance(x, tuple):
        assert len(x) == 2, 'tuple has length %d, not 2' % len(x)
        return x
    else:
        return (x, x)

class Image(object):
    def __init__(self, size=None, img=None, unit=1):
        self.unit = t2(unit)

        if img is not None:
            iw, ih = img.size
            ux, uy = self.unit
            assert iw % ux == 0, 'image width is not divisible by unit'
            assert ih % uy == 0, 'image height is not divisible by unit'
            self.size = (iw // ux, ih // uy)
            self._img = img
        else:
            px_size = tuple(u * s for u,s in zip(self.unit, size))
            self.size = size
            self._img = img or PIL.Image.new('RGBA', px_size)

        self.px_size = self._img.size

    def raw(self):
        return self._img

    def modify(self, f, unit=None):
        unit = t2(unit) if unit else self.unit
        new_img = self._img.copy()
        new_img = f(new_img) or new_img
        return Image(img=new_img, unit=unit)

    def with_unit(self, unit):
        return Image(img=self._img, unit=unit)

    def extract(self, pos, size=1, unit=None):
        x, y = pos
        w, h = t2(size)
        unit = t2(unit) if unit else self.unit
        ux, uy = unit

        new_img = self._img.crop((x * ux, y * uy, (x + w) * ux, (y + h) * uy))
        return Image(img=new_img, unit=unit)

    def chop_list(self, names, unit=None):
        dct = {}
        points = ((x,y) for y in range(self.size[1]) for x in range(self.size[0]))
        for pos, name in zip(points, names):
            if name is None:
                continue
            dct[name] = self.extract(pos, unit=unit)
        return dct

    def chop_grid(self, names, unit=None):
        dct = {}
        for y, row in enumerate(names):
            for x, name in enumerate(row):
                if name is None:
                    continue
                dct[name] = self.extract((x, y), unit=unit)
        return dct

    def chop(self, name_dct, unit=None):
        dct = {}
        for name, pos in name_dct.items():
            dct[name] = self.extract(pos, unit=unit)
        return dct

    def scale(self, size, unit=None, smooth=False):
        unit = t2(unit) if unit else self.unit
        w, h = size
        ux, uy = unit
        px_w, px_h = (w * ux, h * uy)

        if smooth:
            if px_w < self.raw().size[0]:
                resample = PIL.Image.ANTIALIAS
            else:
                resample = PIL.Image.BICUBIC
        else:
            resample = PIL.Image.NEAREST
        new_img = self.raw().resize((px_w, px_h), resample)

        return Image(img=new_img, unit=unit)

    def stack(self, imgs):
        assert all(i.size == self.size and i.unit == self.unit for i in imgs), \
                'stacked images must match in size and unit'
        new_img = self._img.copy()
        for i in imgs:
            new_img.paste(i.raw(), (0, 0), i.raw())
        return Image(img=new_img, unit=self.unit)

    def pad(self, size, offset=None, unit=1):
        sx, sy = t2(size)
        ux, uy = t2(unit)

        px_x = sx * ux
        px_y = sy * uy

        if offset is None:
            ox = px_x // 2 - self.px_size[0] // 2
            oy = px_y // 2 - self.px_size[1] // 2
            offset = (ox, oy)

        new_img = PIL.Image.new('RGBA', (px_x, px_y))
        new_img.paste(self.raw(), offset)
        return Image(img=new_img, unit=unit)


def stack(imgs):
    imgs[0].stack(imgs[1:])

def stack_offset(layers, unit=1):
    ux, uy = t2(unit)

    # Get the size needed to contain all layers
    w, h = 0, 0
    for (ox, oy), img in layers:
        img_w, img_h = img.raw().size
        w = max(w, ox * ux + img_w)
        h = max(h, oy * uy + img_h)

    # Round up to a multiple of `unit`
    w = (w + ux - 1) // ux * ux
    h = (h + uy - 1) // uy * uy

    # Construct new image
    new_img = PIL.Image.new('RGBA', (w, h))
    for (ox, oy), img in layers:
        new_img.paste(img.raw(), (ox * ux, oy * uy))
    return Image(img=new_img, unit=(ux, uy))


# Cache of images indexed by their real file path.
LOAD_CACHE = {}

def _load_image(full_path, unit):
    if full_path not in LOAD_CACHE:
        LOAD_CACHE[full_path] = Image(img=PIL.Image.open(full_path), unit=unit or 1)
    return LOAD_CACHE[full_path]

def load(path, mod=None, unit=None):
    mod = mod or util.get_caller_mod_name()
    full_path = files.find(path, mod)
    if full_path is None:
        raise KeyError('image not found: %r (in mod %r)' % (path, mod))
    return _load_image(full_path, unit)

def loader(path=None, mod=None, unit=None):
    mod = mod or util.get_caller_mod_name()
    if path is None:
        def f(name, unit=1):
            return load(name, mod, unit)
        return f
    else:
        def f(name, unit=1):
            return load(os.path.join(path, name), mod, unit)
        return f
