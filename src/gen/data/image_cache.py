import functools
import hashlib
import pickle
import os
import sys

import PIL


IMAGE_CACHE = {}
NEW_IMAGE_CACHE = {}
# TODO: The current cache management scheme drops intermediate images after the
# first run.  That is, the first run adds all the intermediate images to the
# cache, but the second run, seeing that only the final products are used,
# drops the intermediate images.  I think this is suboptimal, but I'm not sure...

COMPUTE_CACHE = {}
NEW_COMPUTE_CACHE = {}

@functools.lru_cache(128)
def _cached_mtime(path):
    return os.path.getmtime(path)

class CachedImage(object):
    """An immutable wrapper around PIL.Image that allows for caching of
    intermediate images."""
    def __init__(self, size, desc, inputs):
        self.size = size
        self._desc = (type(self), desc, tuple(i._desc for i in inputs))
        self._raw = None

    def _realize(self):
        raise RuntimeError('CachedImage subclass must implement _realize()')

    def raw(self):
        global IMAGE_CACHE, NEW_IMAGE_CACHE
        img = self._raw
        if img is not None:
            return img

        img = IMAGE_CACHE.get(self._desc)
        if img is not None:
            assert img.size == self.size, 'cache contained an image of the wrong size'
            self._raw = img
            NEW_IMAGE_CACHE[self._desc] = img
            return img

        img = self._realize()
        assert img is not None, '_realize() must return a PIL.Image, not None'
        assert img.size == self.size, '_realize() returned an image of the wrong size'
        self._raw = img
        IMAGE_CACHE[self._desc] = img
        NEW_IMAGE_CACHE[self._desc] = img
        return img

    def compute(self, f):
        code_file = sys.modules[f.__module__].__file__
        code_time = _cached_mtime(code_file)
        k = (self._desc, f.__module__, f.__qualname__, code_file, code_time)

        if k in COMPUTE_CACHE:
            result = COMPUTE_CACHE[k]
        else:
            result = f(self.raw())
            COMPUTE_CACHE[k] = result

        if k not in NEW_COMPUTE_CACHE:
            NEW_COMPUTE_CACHE[k] = result

        return result

    def desc(self):
        return self._desc

    @staticmethod
    def blank(size):
        return BlankImage(size)

    @staticmethod
    def open(filename):
        return FileImage(filename)

    def modify(self, f, size=None, desc=None):
        if desc is None:
            desc = '%s.%s' % (f.__module__, f.__qualname__)
        return ModifiedImage(self, f, size or self.size, desc)

    def crop(self, bounds):
        return CroppedImage(self, bounds)

    def resize(self, size, resample=0):
        return ResizedImage(self, size, resample)

    def stack(self, imgs):
        return StackedImage((self,) + tuple(imgs))

    def pad(self, size, offset):
        return PaddedImage(self, size, offset)

    def get_bounds(self):
        return self.compute(lambda i: i.getbbox())

class BlankImage(CachedImage):
    def __init__(self, size):
        super(BlankImage, self).__init__(size, size, ())

    def _realize(self):
        return PIL.Image.new('RGBA', self.size)

class ConstImage(CachedImage):
    def __init__(self, img):
        h = hashlib.sha1(bytes(x for p in img.getdata() for x in p)).hexdigest()
        super(ConstImage, self).__init__(img.size, (img.size, h), ())
        self._raw = img

    def _realize(self):
        assert False, 'ConstImage already sets self._raw, should be no need to call _realize()'

class FileImage(CachedImage):
    def __init__(self, filename):
        mtime = os.path.getmtime(filename)
        img = PIL.Image.open(filename)
        super(FileImage, self).__init__(img.size, (filename, mtime), ())
        self._raw = img

    def _realize(self):
        assert False, 'FileImage already sets self._raw, should be no need to call _realize()'

class ModifiedImage(CachedImage):
    def __init__(self, img, f, size, desc):
        code_file = sys.modules[f.__module__].__file__
        code_time = _cached_mtime(code_file)

        super(ModifiedImage, self).__init__(size, (desc, size, code_time), (img,))
        self.orig = img
        self.f = f

    def _realize(self):
        img = self.orig.raw().copy()
        return self.f(img) or img

class CroppedImage(CachedImage):
    def __init__(self, img, bounds):
        x0, y0, x1, y1 = bounds
        w = x1 - x0
        h = y1 - y0

        super(CroppedImage, self).__init__((w, h), bounds, (img,))

        self.orig = img
        self.bounds = bounds

    def _realize(self):
        return self.orig.raw().crop(self.bounds)

class ResizedImage(CachedImage):
    def __init__(self, img, size, resample=0):
        super(ResizedImage, self).__init__(size, (size, resample), (img,))

        self.orig = img
        # self.size already set
        self.resample = resample

    def _realize(self):
        return self.orig.raw().resize(self.size, self.resample)

class StackedImage(CachedImage):
    def __init__(self, imgs):
        assert all(i.size == imgs[0].size for i in imgs)
        super(StackedImage, self).__init__(imgs[0].size, (), imgs)
        self.imgs = imgs

    def _realize(self):
        img = self.imgs[0].raw().copy()
        for i in self.imgs[1:]:
            layer_img = i.raw()
            img.paste(layer_img, (0, 0), layer_img)
        return img

class PaddedImage(CachedImage):
    def __init__(self, img, size, offset):
        super(PaddedImage, self).__init__(size, (size, offset), (img,))
        self.orig = img
        # self.size already set
        self.offset = offset

    def _realize(self):
        orig_img = self.orig.raw()
        img = PIL.Image.new(orig_img.mode, self.size)
        img.paste(orig_img, self.offset, orig_img)
        return img


def load_cache(f):
    global IMAGE_CACHE, COMPUTE_CACHE
    i, c = pickle.load(f)
    IMAGE_CACHE.update(i)
    COMPUTE_CACHE.update(c)

def dump_cache(f):
    global NEW_IMAGE_CACHE, NEW_COMPUTE_CACHE
    for k, v in NEW_IMAGE_CACHE.items():
        if type(v) is not PIL.Image.Image:
            new_v = v.copy()
            new_v.load()
            NEW_IMAGE_CACHE[k] = new_v
    blob = (NEW_IMAGE_CACHE, NEW_COMPUTE_CACHE)
    pickle.dump(blob, f, -1)
