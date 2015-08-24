use std::collections::{HashMap, HashSet};
use std::fmt;

use libserver_types::*;


struct Triangle {
    a: V2,
    b: V2,
    c: V2,
    minors: [i64; 4],
}

impl fmt::Debug for Triangle {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        (self.a, self.b, self.c).fmt(f)
    }
}

impl Triangle {
    pub fn new(a: V2, b: V2, c: V2) -> Triangle {
        let (a, b, c) = sort_ccw(a, b, c);
        let minors = calc_minors(a, b, c);
        Triangle {
            a: a,
            b: b,
            c: c,
            minors: minors,
        }
    }

    pub fn circumcircle_contains(&self, p: V2) -> bool {
        let (dx, dy) = (p.x as i64, p.y as i64);
        let x0 = dx;
        let x1 = dy;
        let x2 = dx * dx + dy * dy;
        let x3 = 1;

        let det = self.minors[0] * x0 +
                  self.minors[1] * x1 +
                  self.minors[2] * x2 +
                  self.minors[3] * x3;
        det > 0
    }
}

/// Given the three vertices of a triangle, sort them into counterclockwise order.
fn sort_ccw(a: V2, b: V2, c: V2) -> (V2, V2, V2) {
    let ab = b - a;
    let ac = c - a;

    // Rotate AB by 90 degrees to the left.
    let left = V2::new(-ab.y, ab.x);

    // Dot `left` with AC.  If the result is positive, then AC points to the left of AB, and the
    // vertices ABC are already sorted counterclockwise.
    //       C---B
    //        \  |
    //         \ |
    //          \|
    // left <--- A
    if ac.dot(left) > 0 {
        (a, b, c)
    } else {
        (a, c, b)
    }
}

/// Precompute part of the determinant of the matrix
///     | Ax  Ay  (Ax^2 + Ay^2)  1 |
///     | Bx  By  (Bx^2 + By^2)  1 |
///     | Cx  Cy  (Cx^2 + Cy^2)  1 |
///     | Dx  Dy  (Dx^2 + Dy^2)  1 |
/// Specifically, we compute the four minors along the D row, so we can later quickly compute the
/// full determinant for any D.
fn calc_minors(a: V2, b: V2, c: V2) -> [i64; 4] {
    let (ax, ay) = (a.x as i64, a.y as i64);
    let (bx, by) = (b.x as i64, b.y as i64);
    let (cx, cy) = (c.x as i64, c.y as i64);

    let (m00, m01, m02) = (ax,  ay,  ax * ax + ay * ay);
    let (m10, m11, m12) = (bx,  by,  bx * bx + by * by);
    let (m20, m21, m22) = (cx,  cy,  cx * cx + cy * cy);

    let x0 = det3(m01, m02, 1, m11, m12, 1, m21, m22, 1);
    let x1 = det3(m00, m02, 1, m10, m12, 1, m20, m22, 1);
    let x2 = det3(m00, m01, 1, m10, m11, 1, m20, m21, 1);
    let x3 = det3(m00, m01, m02, m10, m11, m12, m20, m21, m22);

    [-x0, x1, -x2, x3]
}

fn det3(m00: i64, m01: i64, m02: i64,
        m10: i64, m11: i64, m12: i64,
        m20: i64, m21: i64, m22: i64) -> i64 {
      m00 * m11 * m22
    + m01 * m12 * m20
    + m02 * m10 * m21
    - m20 * m11 * m02
    - m21 * m12 * m00
    - m22 * m10 * m01
}

fn bounding_triangle(points: &[V2]) -> [V2; 3] {
    let min_x = points.iter().map(|p| p.x).min().unwrap();
    let min_y = points.iter().map(|p| p.y).min().unwrap();
    let max_x = points.iter().map(|p| p.x).max().unwrap();
    let max_y = points.iter().map(|p| p.y).max().unwrap();

    // The adjustment by 10 ensures that the bounding triangle vertices don't coincide with any
    // real point.
    let min = V2::new(min_x, min_y) - scalar(10);
    let max = V2::new(max_x, max_y) + scalar(10);
    let delta = max - min;

    let a = min;
    let b = min + V2::new(2 * delta.x, 0);
    let c = min + V2::new(0, 2 * delta.y);
    [a, b, c]
}

/// Compute the Delaunay triangulation of a set of points.  The result is list of edges, each
/// represented as a pair of indices into the input list.
pub fn triangulate(points: &[V2]) -> Vec<(u16, u16)> {
    let extra_points = bounding_triangle(points);

    let mut tris = Vec::new();
    tris.push(Triangle::new(extra_points[0], extra_points[1], extra_points[2]));

    let mut bad_tri_idxs = Vec::new();
    let mut bad_edges = HashMap::new();

    for &p in points {
        bad_tri_idxs.clear();
        for (i, tri) in tris.iter().enumerate() {
            if tri.circumcircle_contains(p) {
                bad_tri_idxs.push(i)
            }
        }

        bad_edges.clear();
        {
            let mut record_edge = |a: V2, b: V2| {
                let edge = 
                    if b.x < a.x || (b.x == a.x && b.y < a.y) {
                        (b, a)
                    } else {
                        (a, b)
                    };
                let entry = bad_edges.entry(edge);
                *entry.or_insert(0) += 1;
            };

            for &i in &bad_tri_idxs {
                let tri = &tris[i];
                record_edge(tri.a, tri.b);
                record_edge(tri.a, tri.c);
                record_edge(tri.b, tri.c);
            }
        }

        bad_tri_idxs.sort();
        // swap_remove always swaps with a later index.  Iterate in reverse so that we always swap
        // into slots that have already been processed.
        for &i in bad_tri_idxs.iter().rev() {
            tris.swap_remove(i);
        }

        for (&(a, b), &count) in &bad_edges {
            if count == 1 {
                tris.push(Triangle::new(a, b, p));
            }
        }
    }

    let point_map = points.iter().enumerate().map(|(i, &p)| (p, i as u16))
                          .collect::<HashMap<_, _>>();
    let mut edges = HashSet::new();
    {
        let mut record_edge = |a, b| {
            if let (Some(&i), Some(&j)) = (point_map.get(&a), point_map.get(&b)) {
                let edge = if j < i { (j, i) } else { (i, j) };
                edges.insert(edge);
            }
        };
        for tri in &tris {
            record_edge(tri.a, tri.b);
            record_edge(tri.a, tri.c);
            record_edge(tri.b, tri.c);
        }
    }

    edges.into_iter().collect::<Vec<_>>()
}
