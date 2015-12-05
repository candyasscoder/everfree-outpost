import functools

from outpost_data.core.consts import *
from outpost_data.core.structure import Model, Model2


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
    return verts_front(x, y, z) + verts_top(x, y, z)

def model_builder(f):
    cache = {}
    @functools.wraps(f)
    def g(*args):
        if args not in cache:
            cache[args] = Model(f(*args))
        return cache[args]
    return g

top = model_builder(verts_top)
front = model_builder(verts_front)
bottom = model_builder(verts_bottom)
solid = model_builder(verts_solid)


TREE = Model(
        quad_z(0, 0, 128, 0, 64) +
        quad((32, 32,   0), (64, 64,   0),
             (64, 64, 128), (32, 32, 128)) +
        quad((64, 64,   0), (96, 32,   0),
             (96, 32, 128), (64, 64, 128)) +
        quad((32, 32, 128), (64, 64, 128),
             (96, 32, 128), (64,  0, 128)))

STUMP = Model(
        quad_z(0, 0, 128, 0, 64) +
        quad((32, 32,   0), (64, 64,   0),
             (64, 64,  32), (32, 32,  32)) +
        quad((64, 64,   0), (96, 32,   0),
             (96, 32,  32), (64, 64,  32)) +
        quad((32, 32,  32), (64, 64,  32),
             (96, 32,  32), (64,  0,  32)))

def _mk_wall_models():
    def front(l, r, t, b):
        # Note: `t` is ignored
        verts = []
        if b:
            verts.extend(quad_y(32, 12, 20,  0, 64))
            if l:
                verts.extend(quad_y(20,  0, 12,  0, 64))
            if r:
                verts.extend(quad_y(20, 20, 32,  0, 64))
        else:
            x0 = 0 if l else 12
            x1 = 32 if r else 20
            verts.extend(quad_y(20, x0, x1,  0, 64))
        return verts

    def top(l, r, t, b):
        verts = []
        if l or r:
            x0 = 0 if l else 12
            x1 = 32 if r else 20
            verts.extend(quad_z(64, x0, x1, 12, 20))
            if t:
                verts.extend(quad_z(64, 12, 20,  0, 12))
            if b:
                verts.extend(quad_z(64, 12, 20, 20, 32))
        else:
            y0 = 0 if t else 12
            y1 = 32 if b else 20
            verts.extend(quad_z(64, 12, 20, y0, y1))
        return verts

    def wall(n, e, s, w):
        return Model(front(w, e, n, s) + top(w, e, n, s))

    m = {}
    m['edge/horiz'] =   wall(0, 1, 0, 1)
    m['edge/vert'] =    wall(1, 0, 1, 0)
    m['corner/nw'] =    wall(0, 1, 1, 0)
    m['corner/ne'] =    wall(0, 0, 1, 1)
    m['corner/se'] =    wall(1, 0, 0, 1)
    m['corner/sw'] =    wall(1, 1, 0, 0)
    m['tee/n'] =        wall(1, 1, 0, 1)
    m['tee/e'] =        wall(1, 1, 1, 0)
    m['tee/s'] =        wall(0, 1, 1, 1)
    m['tee/w'] =        wall(1, 0, 1, 1)
    m['cross'] =        wall(1, 1, 1, 1)

    m['door'] = Model(
            quad_y(20,  0, 96,  0, 64) +
            quad_z(64,  0, 96,  0, 20))

    return m

def _mk_wall_models2(wall_models):
    m = {}
    for k,v in wall_models.items():
        if k == 'door':
            bounds = ((0, 0, 0), (3 * TILE_SIZE, 1 * TILE_SIZE, 2 * TILE_SIZE))
        else:
            bounds = ((0, 0, 0), (1 * TILE_SIZE, 1 * TILE_SIZE, 2 * TILE_SIZE))
        m[k] = Model2(v.to_mesh(), bounds)
    return m

WALL = _mk_wall_models()
WALL2 = _mk_wall_models2(WALL)
