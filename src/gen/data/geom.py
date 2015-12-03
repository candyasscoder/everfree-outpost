from collections import namedtuple
import math

def add(a, b):
    if isinstance(b, tuple):
        return tuple(ax + bx for ax,bx in zip(a, b))
    else:
        return tuple(ax + b for ax in a)

def sub(a, b):
    if isinstance(b, tuple):
        return tuple(ax - bx for ax,bx in zip(a, b))
    else:
        return tuple(ax - b for ax in a)

def mul(a, c):
    return tuple(ax * c for ax in a)

def div(a, c):
    return tuple(ax / c for ax in a)

def dot(a, b):
    return sum(ax * bx for ax,bx in zip(a, b))

def line_plane(a, b, p, n):
    d = sub(b, a)
    l = dot(d, n)
    if l == 0:
        return add(a, mul(d, 0.5))
    r = dot(sub(p, a), n)
    t = r / l
    return add(a, mul(d, t))

def round_nearest(a):
    return tuple(round(ax) for ax in a)

def round_up(a):
    return tuple(math.ceil(ax) for ax in a)

def round_away(a, o):
    d = sub(a, o)
    return add(o, round_up(d))

def slice_triangle(tri, p, n):
    """Slice a 3D triangle using a plane defined by a point and normal.  Parts
    on the `-n` side of the plane will be discarded."""
    a, b, c = tri

    pn = dot(p, n)
    a_pos = dot(a, n) > pn
    b_pos = dot(b, n) > pn
    c_pos = dot(c, n) > pn
    count = int(a_pos) + int(b_pos) + int(c_pos)

    center = round_nearest(div(add(add(a, b), c), 3))

    if count == 3:
        yield tri
    elif count == 0:
        pass
    else:
        ab = round_away(line_plane(a, b, p, n), center)
        bc = round_away(line_plane(b, c, p, n), center)
        ca = round_away(line_plane(c, a, p, n), center)

        if count == 2:
            # Two points are inside, and there are two intersection points.  We
            # need to split the resulting quad into two tris.
            if not a_pos:
                yield (b, c, ca)
                yield (b, ca, ab)
            elif not b_pos:
                yield (c, a, ab)
                yield (c, ab, bc)
            elif not c_pos:
                yield (a, b, bc)
                yield (a, bc, ca)
        elif count == 1:
            # One point is inside, and there are two intersection points.  Just
            # produce the triangle.
            if a_pos:
                yield (a, ab, ca)
            elif b_pos:
                yield (b, bc, ab)
            elif c_pos:
                yield (c, ca, bc)

def slice_tris_box_xv(verts, min_x, max_x, min_v, max_v):
    n1 = (1, 0, 0)
    p1 = (min_x, 0, 0)

    n2 = (-1, 0, 0)
    p2 = (max_x, 0, 0)

    n3 = (0, 1, -1)
    p3 = (0, min_v, 0)

    n4 = (0, -1, 1)
    p4 = (0, max_v, 0)

    for i in range(0, len(verts), 3):
        tri = verts[i : i + 3]
        for t1 in slice_triangle(tri, p1, n1):
            for t2 in slice_triangle(t1, p2, n2):
                for t3 in slice_triangle(t2, p3, n3):
                    for t4 in slice_triangle(t3, p4, n4):
                        yield t4

def project_isometric(p, center):
    px, py, pz = p
    dx = mul((1, -0.5), px)
    dy = mul((1, 0.5), py)
    dz = mul((0, -1), pz)
    return add(add(add(center, dx), dy), dz)

def draw_isometric(verts, draw, center, fill):
    for i in range(0, len(verts), 3):
        a, b, c = verts[i : i + 3]
        p = project_isometric(a, center)
        q = project_isometric(b, center)
        r = project_isometric(c, center)
        draw.line((p, q), fill=fill)
        draw.line((q, r), fill=fill)
        draw.line((r, p), fill=fill)

Vertex = namedtuple('Vertex', ('pos', 'edges', 'tris'))
Edge = namedtuple('Edge', ('verts', 'tris'))
Triangle = namedtuple('Triangle', ('verts', 'edges'))

class Mesh(object):
    def __init__(self):
        self.verts = []
        self.vert_idx = {}
        self.free_verts = []

        self.edges = []
        self.edge_idx = {}
        self.free_edges = []

        self.tris = []
        self.free_tris = []

    def copy(self):
        other = Mesh()

        other.verts = [Vertex(v.pos, v.edges.copy(), v.tris.copy())
                for v in self.verts]
        other.vert_idx = self.vert_idx.copy()
        other.free_verts = self.free_verts.copy()

        other.edges = [Edge(e.verts, e.tris.copy())
                for e in self.edges]
        other.edge_idx = self.edge_idx.copy()
        other.free_edges = self.free_edges.copy()

        other.tris = [Triangle(t.verts, t.edges)
                for t in self.tris]
        other.free_tris = self.free_tris.copy()

        return other

    def _next_vert_idx(self):
        if len(self.free_verts) > 0:
            return self.free_verts.pop()
        else:
            self.verts.append(0)
            return len(self.verts) - 1

    def _next_edge_idx(self):
        if len(self.free_edges) > 0:
            return self.free_edges.pop()
        else:
            self.edges.append(0)
            return len(self.edges) - 1

    def _next_tri_idx(self):
        if len(self.free_tris) > 0:
            return self.free_tris.pop()
        else:
            self.tris.append(0)
            return len(self.tris) - 1

    def _add_vert(self, pos):
        idx = self.vert_idx.get(pos)
        if idx is None:
            idx = self._next_vert_idx()
            self.vert_idx[pos] = idx
            self.verts[idx] = Vertex(pos, set(), set())
        return idx

    def _add_edge(self, i, j):
        if i > j:
            i, j = j, i

        idx = self.edge_idx.get((i, j))
        if idx is None:
            idx = self._next_edge_idx()
            self.edge_idx[(i, j)] = idx
            self.edges[idx] = Edge((i, j), set())
            self.verts[i].edges.add(idx)
            self.verts[j].edges.add(idx)
        return idx

    def add_tri(self, a, b, c):
        i = self._add_vert(a)
        j = self._add_vert(b)
        k = self._add_vert(c)
        i, j, k = sorted((i, j, k))
        if i == j or j == k:
            # They're sorted, remember
            return None

        ij = self._add_edge(i, j)
        jk = self._add_edge(j, k)
        ki = self._add_edge(k, i)
        ij, jk, ki = sorted((ij, jk, ki))

        idx = self._next_tri_idx()
        self.tris[idx] = Triangle((i, j, k), (ij, jk, ki))

        self.verts[i].tris.add(idx)
        self.verts[j].tris.add(idx)
        self.verts[k].tris.add(idx)
        self.edges[ij].tris.add(idx)
        self.edges[jk].tris.add(idx)
        self.edges[ki].tris.add(idx)

        return idx

    def _remove_vert(self, idx):
        v = self.verts[idx]
        del self.vert_idx[v.pos]
        self.verts[idx] = None
        self.free_verts.append(idx)

    def _remove_vert_edge(self, v_idx, e_idx):
        v = self.verts[v_idx]
        v.edges.remove(e_idx)
        if len(v.tris) == 0 and len(v.edges) == 0:
            self._remove_vert(v_idx)

    def _remove_vert_tri(self, v_idx, t_idx):
        v = self.verts[v_idx]
        v.tris.remove(t_idx)
        if len(v.tris) == 0 and len(v.edges) == 0:
            self._remove_vert(v_idx)

    def _remove_edge(self, idx):
        e = self.edges[idx]
        del self.edge_idx[e.verts]
        for v_idx in e.verts:
            self._remove_vert_edge(v_idx, idx)
        self.edges[idx] = None
        self.free_edges.append(idx)

    def _remove_edge_tri(self, e_idx, t_idx):
        e = self.edges[e_idx]
        e.tris.remove(t_idx)
        if len(e.tris) == 0:
            self._remove_edge(e_idx)

    def remove_tri(self, idx):
        t = self.tris[idx]
        for e_idx in t.edges:
            self._remove_edge_tri(e_idx, idx)
        for v_idx in t.verts:
            self._remove_vert_tri(v_idx, idx)
        self.tris[idx] = None
        self.free_tris.append(idx)

    def slice_tri(self, idx, p, n):
        """Slice a 3D triangle using a plane defined by a point and normal.  Parts
        on the `-n` side of the plane will be discarded."""
        if self.tris[idx] is None:
            return

        a, b, c = (self.verts[i].pos for i in self.tris[idx].verts)

        pn = dot(p, n)
        a_pos = dot(a, n) > pn
        b_pos = dot(b, n) > pn
        c_pos = dot(c, n) > pn
        count = int(a_pos) + int(b_pos) + int(c_pos)

        center = round_nearest(div(add(add(a, b), c), 3))

        if count == 3:
            # Entire triangle is inside.  Don't remove anything.
            return
        else:
            self.remove_tri(idx)

        if count == 0:
            # Entire triangle is outside.  Don't add anything.
            return

        ab = round_away(line_plane(a, b, p, n), center)
        bc = round_away(line_plane(b, c, p, n), center)
        ca = round_away(line_plane(c, a, p, n), center)

        if count == 1:
            # One point is inside, and there are two intersection points.  Just
            # produce one triangle.
            if a_pos:
                new_idx = self.add_tri(a, ab, ca)
            elif b_pos:
                new_idx = self.add_tri(b, bc, ab)
            elif c_pos:
                new_idx = self.add_tri(c, ca, bc)

        elif count == 2:
            # Two points are inside, and there are two intersection points.  We
            # need to split the resulting quad into two tris.
            if not a_pos:
                new_idx1 = self.add_tri(b, c, ca)
                new_idx2 = self.add_tri(b, ca, ab)
            elif not b_pos:
                new_idx1 = self.add_tri(c, a, ab)
                new_idx2 = self.add_tri(c, ab, bc)
            elif not c_pos:
                new_idx1 = self.add_tri(a, b, bc)
                new_idx2 = self.add_tri(a, bc, ca)

    def slice(self, p, n):
        for i in range(len(self.tris)):
            self.slice_tri(i, p, n)

    def try_merge(self, idx):
        if idx is None or self.tris[idx] is None:
            return False
        tri = self.tris[idx]

        for e_idx in tri.edges:
            e = self.edges[e_idx]
            if len(e.tris) == 1:
                continue

            v_idx, = (v for v in tri.verts if v not in e.verts)

            other_idx, = (t for t in e.tris if t != idx)
            other_tri = self.tris[other_idx]
            other_v_idx, = (v for v in other_tri.verts if v not in e.verts)

            a = self.verts[v_idx].pos
            b = self.verts[other_v_idx].pos

            ms = [self.verts[e.verts[i]].pos for i in (0, 1)]

            for i, m in enumerate(ms):
                d1 = sub(m, a)
                d2 = sub(b, m)
                dd = dot(d1, d2)
                if abs(dd * dd - dot(d1, d1) * dot(d2, d2)) < 1e-6:
                    self.remove_tri(idx)
                    self.remove_tri(other_idx)
                    self.add_tri(a, b, ms[1 - i])
                    return True

        return False

    def flip_adjacent(self, idx1, idx2):
        if idx1 is None or self.tris[idx1] is None or \
                idx2 is None or self.tris[idx2] is None:
            return

        tri1 = self.tris[idx1]
        tri2 = self.tris[idx2]
        common_vert_idxs = tuple(v for v in tri1.verts if v in tri2.verts)
        common_verts = tuple(self.verts[v].pos for v in common_vert_idxs)
        diff_verts = tuple(self.verts[v].pos
                for v in set(tri1.verts + tri2.verts) if v not in common_vert_idxs)

        self.remove_tri(idx1)
        self.remove_tri(idx2)
        self.add_tri(common_verts[0], *diff_verts)
        self.add_tri(common_verts[1], *diff_verts)

    # NB: expensive, but should be okay since most meshes are small
    def simplify(self):
        keep_going = True
        while keep_going:
            keep_going = False

            for i in range(len(self.tris)):
                keep_going |= self.try_merge(i)

            for i in range(len(self.edges)):
                e = self.edges[i]
                if e is None or len(e.tris) != 2:
                    continue
                idx1, idx2 = e.tris
                self.flip_adjacent(idx1, idx2)
                if self.try_merge(idx1) | self.try_merge(idx2):
                    keep_going = True
                else:
                    # Flip them back
                    self.flip_adjacent(idx1, idx2)

    def iter_verts(self):
        for v in self.verts:
            if v is not None:
                yield v

    def iter_tri_verts(self):
        for t in self.tris:
            if t is None:
                continue
            for i in t.verts:
                v = self.verts[i]
                if v is None:
                    continue
                yield v

    def get_bounds_2d(self, proj):
        vs = [proj(v.pos) for v in self.iter_verts()]
        return ((min(x for x,y in vs),
                 min(y for x,y in vs)),
                (max(x for x,y in vs),
                 max(y for x,y in vs)))

    def draw(self, d, fill='blue'):
        for e in self.edges:
            if e is None:
                continue
            i, j = e.verts
            d.line(self.verts[i].pos + self.verts[j].pos, fill=fill)

    def draw_iso(self, d, center, fill='blue'):
        get_pos = lambda i: project_isometric(self.verts[i].pos, center)
        for e in self.edges:
            if e is None:
                continue
            i, j = e.verts
            d.line(get_pos(i) + get_pos(j), fill=fill)

def clip_xv(mesh, min_x, min_v, max_x, max_v):
    mesh.slice((min_x, 0, 0), (1, 0, 0))
    mesh.slice((max_x, 0, 0), (-1, 0, 0))
    mesh.slice((0, min_v, 0), (0, 1, -1))
    mesh.slice((0, max_v, 0), (0, -1, 1))
    mesh.simplify()

if __name__ == '__main__':
    from PIL import Image, ImageDraw
    img = Image.new('RGBA', (256, 256))
    draw = ImageDraw.Draw(img)

    mesh = Mesh()
    mesh.add_tri((32, 32), (96, 32), (32, 96))
    mesh.add_tri((96, 96), (96, 32), (32, 96))
    mesh = mesh.copy()
    mesh.slice((64, 0, 0), (1, 0, 0))
    mesh.slice((64, 64, 0), (1, 1, 0))

    mesh.draw(draw)
    img.save('test.png')

