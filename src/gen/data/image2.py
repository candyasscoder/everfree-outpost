import os
from outpost_data.core.image_cache import CachedImage

import PIL  # for filter type constants

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
            self._img = img or CachedImage.blank(px_size)

        self.px_size = self._img.size

    def raw(self):
        return self._img

    def modify(self, f, size=None, unit=None):
        unit = t2(unit) if unit else self.unit

        if size is None:
            px_size = self.raw().size
        else:
            size = t2(size)
            px_x = size[0] * unit[0]
            px_y = size[1] * unit[1]
            px_size = (px_x, px_y)

        new_img = self.raw().modify(f, size=px_size)
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
        new_img = self.raw().stack(i.raw() for i in imgs)
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
        else:
            offset = t2(offset)

        new_img = self.raw().pad((px_x, px_y), offset)
        return Image(img=new_img, unit=unit)

    def hash(self):
        return self.raw().compute(util.hash_image)

    def get_bounds(self):
        return self.raw().get_bounds()

    def autocrop(self):
        x0, y0, x1, y1 = self.get_bounds()
        img = self.extract((x0, y0), size=(x1 - x0, y1 - y0), unit=1)
        return (img, (x0, y0))

class Anim(object):
    def __init__(self, frames, rate, oneshot=False):
        self._frames = frames
        self.length = len(frames)
        self.rate = rate
        self.oneshot = oneshot

        self.unit = self._frames[0].unit
        self.size = self._frames[0].size
        self.px_size = self._frames[0].px_size
        assert all(f.unit == self.unit for f in self._frames), \
                'frame units do not match'
        assert all(f.size == self.size for f in self._frames), \
                'frame sizes do not match'

    def _with_frames(self, f):
        return Anim(list(f), self.rate, self.oneshot)

    def _map_frames(self, name, args, kwargs):
        return self._with_frames([getattr(i, name)(*args, **kwargs) for i in self._frames])

    def raw(self):
        return self._frames[0].raw()

    def flatten(self):
        w,h = self.px_size
        out_w = w * self.length
        layers = []
        for i, img in enumerate(self._frames):
            layers.append(img.pad((out_w, h), offset=(i * w, 0)))
        return stack(layers)

    def modify(self, *args, **kwargs):
        return self._map_frames('modify', args, kwargs)

    def with_unit(self, *args, **kwargs):
        return self._map_frames('with_unit', args, kwargs)

    def extract(self, *args, **kwargs):
        return self._map_frames('extract', args, kwargs)

    def chop_list(self, *args, **kwargs):
        return self._map_frames('chop_list', args, kwargs)

    def chop_grid(self, *args, **kwargs):
        return self._map_frames('chop_grid', args, kwargs)

    def chop(self, *args, **kwargs):
        return self._map_frames('chop', args, kwargs)

    def scale(self, *args, **kwargs):
        return self._map_frames('scale', args, kwargs)

    def stack(self, *args, **kwargs):
        return self._map_frames('stack', args, kwargs)

    def pad(self, *args, **kwargs):
        return self._map_frames('pad', args, kwargs)

    def autocrop(self):
        x0, y0, x1, y1 = self._frames[0].get_bounds()
        for f in self._frames[1:]:
            x0_, y0_, x1_, y1_ = f.get_bounds()
            x0 = min(x0, x0_)
            y0 = min(y0, y0_)
            x1 = max(x1, x1_)
            y1 = max(y1, y1_)

        offset = (x0, y0)
        size = (x1 - x0, y1 - y0)
        img = Anim([f.extract(offset, size=size, unit=1) for f in self._frames],
                self.rate, self.oneshot)
        return (img, offset)

def stack(imgs):
    return imgs[0].stack(imgs[1:])


# Cache of images indexed by their real file path.
LOAD_CACHE = {}

def _load_image(full_path, unit):
    if full_path not in LOAD_CACHE:
        LOAD_CACHE[full_path] = Image(img=CachedImage.open(full_path), unit=unit or 1)
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
        def f(name, unit=unit):
            return load(name, mod, unit)
        return f
    else:
        def f(name, unit=unit):
            return load(os.path.join(path, name), mod, unit)
        return f
