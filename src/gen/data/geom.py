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

