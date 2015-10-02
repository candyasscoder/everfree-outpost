from outpost_data.core.builder import mk_model
from outpost_data.outpost.lib import models

def init():
    mk_model('tree', models.verts_solid(4, 2, 4))
    mk_model('stump', models.verts_solid(4, 2, 1))

    def front(l, r, t, b):
        # Note: `t` is ignored
        verts = []
        if b:
            verts.extend(models.quad_y(32, 12, 20,  0, 64))
            if l:
                verts.extend(models.quad_y(20,  0, 12,  0, 64))
            if r:
                verts.extend(models.quad_y(20, 20, 32,  0, 64))
        else:
            x0 = 0 if l else 12
            x1 = 32 if r else 20
            verts.extend(models.quad_y(20, x0, x1,  0, 64))
        return verts

    def top(l, r, t, b):
        verts = []
        if l or r:
            x0 = 0 if l else 12
            x1 = 32 if r else 20
            verts.extend(models.quad_z(64, x0, x1, 12, 20))
            if t:
                verts.extend(models.quad_z(64, 12, 20,  0, 12))
            if b:
                verts.extend(models.quad_z(64, 12, 20, 20, 32))
        else:
            y0 = 0 if t else 12
            y1 = 32 if b else 20
            verts.extend(models.quad_z(64, 12, 20, y0, y1))
        return verts

    def wall(n, e, s, w):
        return front(w, e, n, s) + top(w, e, n, s)

    mk_model('wall/edge/horiz',     wall(0, 1, 0, 1))
    mk_model('wall/edge/vert',      wall(1, 0, 1, 0))
    mk_model('wall/corner/nw',      wall(0, 1, 1, 0))
    mk_model('wall/corner/ne',      wall(0, 0, 1, 1))
    mk_model('wall/corner/se',      wall(1, 0, 0, 1))
    mk_model('wall/corner/sw',      wall(1, 1, 0, 0))
    mk_model('wall/tee/n',          wall(1, 1, 0, 1))
    mk_model('wall/tee/e',          wall(1, 1, 1, 0))
    mk_model('wall/tee/s',          wall(0, 1, 1, 1))
    mk_model('wall/tee/w',          wall(1, 0, 1, 1))
    mk_model('wall/cross',          wall(1, 1, 1, 1))

    mk_model('wall/door',
            models.quad_y(20,  0, 96,  0, 64) +
            models.quad_z(64,  0, 96, 12, 20))
