from collections import defaultdict, namedtuple
import random

import numpy as np
from PIL import Image, ImageDraw

try:
    from outpost_savegame import V2
except ImportError:
    class V2(object):
        def __init__(self, x, y):
            self.x = x
            self.y = y

        def __add__(self, other):
            return V2(self.x + other.x, self.y + other.y)

        def __sub__(self, other):
            return V2(self.x - other.x, self.y - other.y)

        def __mul__(self, other):
            return V2(self.x * other.x, self.y * other.y)

        def __hash__(self):
            return hash((self.x, self.y))


def dist2(a, b):
    d = a - b
    return d.x * d.x + d.y * d.y


# Poisson disk sampling

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
            if dist2(pos, p) < self.min2:
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
                d2 = dist2(V2(0, 0), off)
                if d2 >= self.min2 and d2 <= self.max2:
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
    '''Fill a grid of the given `size` with randomly placed points.  Every
    point is at least `spacing` distance from each of its neighbors.  If
    `init_points` is provided, each point in that list will be included in the
    output (assuming no points in the list are closer than `spacing`).'''
    samp = DiskSampler(rng, size, spacing)
    samp.generate(30, init_points)
    return samp.points


# Delaunay triangulation.

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
    '''Run Delaunay triangulation over a collection of points.  Returns the
    mesh in adjacency list format (`v2 in result[v1]` iff `v1` and `v2` are
    connected by an edge in the mesh, where `v1` and `v2` are `V2` instances.
    '''
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


# Main

def main():
    # Generate the graph.  Place some random points and connect them using
    # triangles.
    rng = random.Random(12345)
    points = disk_sample(V2(256, 256), 8, rng=rng)
    edge_map = triangulate(points)
    edges = list(set((s, t) for s,ts in edge_map.items() for t in ts))

    # Generate a random blob by choosing vertices from the graph.

    # Dict mapping each vertex position to a count of the number of blob
    # vertices connected to that vertex.
    adj_map = defaultdict(lambda: 0)
    # Set of points chosen to be part of the blob.
    chosen = set()
    # Set of all points adjacent to the blob.
    pending = set()

    # Start with a seed point near the center of the region.
    center = V2(128, 128)
    pending.add(sorted(points, key=lambda p: dist2(p, center))[0])

    # Stop after choosing 30 points, or if there are no more points to
    # consider.
    while len(pending) > 0 and len(chosen) < 30:
        # Consider all the adjacent vertices, and maximum number of edges
        # connecting one of those vertices back to the blob.  Choose a point
        # that is as well-connected as the maximum.
        m = max(adj_map[n] for n in pending)
        highest = list(n for n in pending if adj_map[n] == m)
        cur = rng.choice(highest)

        # Add the chosen point to the blob.
        pending.remove(cur)
        chosen.add(cur)

        # The blob has grown, so update `adj_map` and `pending`.
        for n in edge_map[cur]:
            if n not in chosen:
                adj_map[n] += 1
                pending.add(n)


        # Everything after this is just for producing the images.  It doesn't
        # affect the result of the algorithm.

        m = max(adj_map[n] for n in pending)

        img = Image.new('RGBA', (256, 256))
        img.paste('black')
        d = ImageDraw.Draw(img)
        for s,t in edges:
            if s in chosen and t in chosen:
                fill = 'white'
            else:
                fill = (64, 64, 64)
            d.line((s.x, s.y, t.x, t.y), fill=fill)
        for p in points:
            if p in chosen:
                fill = 'white'
            elif p in pending:
                if adj_map[p] == m:
                    fill = (0, 255, 0)
                else:
                    fill = (0, 128, 0)
            else:
                continue
            d.rectangle((p.x - 1, p.y - 1, p.x + 2, p.y + 2), fill=fill)
        d.text((5, 5), 'improved')
        img.save('work/improved-%03d.png' % len(chosen))

if __name__ == '__main__':
    main()
