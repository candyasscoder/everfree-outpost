from collections import defaultdict, namedtuple
import random

import numpy as np
from PIL import Image, ImageDraw

from outpost_savegame import V2, V3


class DiskSampler(object):
    def __init__(self, rng, size, spacing):
        self.rng = rng
        self.points = []
        self.queue_start = 0
        self.size = size
        self.spacing = spacing
        self.min2 = spacing * spacing
        self.max2 = 4 * self.min2

    def check_spacing(self, pos):
        if pos.x < 0 or pos.x >= self.size.x or \
                pos.y < 0 or pos.y >= self.size.y:
            return False

        for p in self.points:
            d = pos - p
            dist2 = d.x * d.x + d.y * d.y
            if dist2 < self.min2:
                return False
        return True

    def try_place(self, pos):
        if self.check_spacing(pos):
            self.points.append(pos)

    def place_nearby(self, pos, tries):
        for i in range(tries):
            candidate = pos
            for j in range(1000):
                x = self.rng.randrange(-self.spacing, self.spacing + 1)
                y = self.rng.randrange(-self.spacing, self.spacing + 1)
                off = V2(x, y)
                dist2 = off.x * off.x + off.y * off.y
                if dist2 >= self.min2 and dist2 <= self.max2:
                    candidate = pos + off
                    break
            self.try_place(candidate)

    def generate(self, tries, init_points=[]):
        for p in init_points:
            self.try_place(p)

        if len(self.points) == 0:
            x = self.rng.randrange(0, self.size.x)
            y = self.rng.randrange(0, self.size.y)
            self.try_place(V2(x, y))

        while self.queue_start < len(self.points):
            p = self.choose_one()
            self.place_nearby(p, tries)

    def choose_one(self):
        idx = self.rng.randrange(self.queue_start, len(self.points))
        result = self.points[idx]
        self.points[idx] = self.points[self.queue_start]
        self.points[self.queue_start] = result
        self.queue_start += 1
        return result

def disk_sample(size, spacing, init_points=[], rng=random):
    samp = DiskSampler(rng, size, spacing)
    samp.generate(30, init_points)
    return samp.points


Triangle = namedtuple('Triangle', ('a', 'b', 'c'))

def mk_triangle(a, b, c):
    a, b, c = sort_ccw(a, b, c)
    return Triangle(a, b, c)

def sort_ccw(a, b, c):
    ab = b - a
    ac = c - a
    left = V2(-ab.y, ab.x)

    if ac.x * left.x + ac.y * left.y > 0:
        return (a, b, c)
    else:
        return (a, c, b)

def circumcircle_contains(tri, p):
    a, b, c = tri
    mat = [
            [a.x, a.y, a.x * a.x + a.y * a.y, 1],
            [b.x, b.y, b.x * b.x + b.y * b.y, 1],
            [c.x, c.y, c.x * c.x + c.y * c.y, 1],
            [p.x, p.y, p.x * p.x + p.y * p.y, 1],
            ]
    return np.linalg.det(mat) > 0

def triangulate(points):
    min_x = min(p.x for p in points) - 10
    max_x = max(p.x for p in points) + 10
    min_y = min(p.y for p in points) - 10
    max_y = max(p.y for p in points) + 10
    tris = [mk_triangle(
        V2(min_x, min_y),
        V2(min_x + 2 * (max_x - min_x), min_y),
        V2(min_x, min_y + 2 * (max_y - min_y)),
        )]

    for p in points:
        bad_tri_idxs = []
        for i, tri in enumerate(tris):
            if circumcircle_contains(tri, p):
                bad_tri_idxs.append(i)

        bad_edges = defaultdict(lambda: 0)
        def record_edge(a, b):
            if b.x < a.x or (b.x == a.x and b.y < a.y):
                a, b = b, a
            bad_edges[(a, b)] += 1
        for i in bad_tri_idxs:
            tri = tris[i]
            record_edge(tri.a, tri.b)
            record_edge(tri.a, tri.c)
            record_edge(tri.b, tri.c)

        for i in sorted(bad_tri_idxs, reverse=1):
            tris[i] = tris[-1]
            tris.pop()

        for (a, b), count in bad_edges.items():
            if count == 1:
                tris.append(mk_triangle(a, b, p))

    point_set = set(points)
    edge_map = defaultdict(set)
    def record_edge(a, b):
        if a not in point_set or b not in point_set:
            return
        edge_map[a].add(b)
        edge_map[b].add(a)
    for tri in tris:
        record_edge(tri.a, tri.b)
        record_edge(tri.a, tri.c)
        record_edge(tri.b, tri.c)
    return edge_map


def main():
    points = disk_sample(V2(256, 256), 16, rng=random.Random(12345))
    edge_map = triangulate(points)
    edges = list(set((s, t) for s,ts in edge_map.items() for t in ts))
    img = Image.new('RGBA', (256, 256))
    d = ImageDraw.Draw(img)
    for p in points:
        d.point((p.x, p.y))
    for s,t in edges:
        print('  %d,%d - %d,%d' % (s.x, s.y, t.x, t.y))
        d.line((s.x, s.y, t.x, t.y))
    img.show()


if __name__ == '__main__':
    main()
