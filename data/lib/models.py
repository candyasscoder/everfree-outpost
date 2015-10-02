import functools

from outpost_data.core.consts import *
from outpost_data.core.builder import INSTANCE, mk_model


def quad(a, b, c, d):
    return [a, b, c, a, c, d]

def quad_y(y, x0, x1, z0, z1):
    return quad(
            # CCW winding, if x/z0 < x/z1
            (x0, y, z0),
            (x1, y, z0),
            (x1, y, z1),
            (x0, y, z1))

def quad_z(z, x0, x1, y0, y1):
    return quad(
            # CCW winding
            (x0, y0, z),
            (x0, y1, z),
            (x1, y1, z),
            (x1, y0, z))

def verts_top(x, y, z):
    return quad_z(z * TILE_SIZE,
            0, x * TILE_SIZE,
            0, y * TILE_SIZE)

def verts_front(x, y, z):
    return quad_y(y * TILE_SIZE,
            0, x * TILE_SIZE,
            0, z * TILE_SIZE)

def verts_bottom(x, y):
    return quad_z(0,
            0, x * TILE_SIZE,
            0, y * TILE_SIZE)

def verts_solid(x, y, z):
    return front(x, y, z) + top(x, y, z)

def model_builder(f):
    @functools.wraps(f)
    def g(*args):
        k = '%s_%s' % (f.__name__, '_'.join(str(x) for x in args))
        if k not in INSTANCE.models:
            mk_model(k, f(*args))
        return k
    return g

top = model_builder(verts_top)
front = model_builder(verts_front)
bottom = model_builder(verts_bottom)
solid = model_builder(verts_solid)
