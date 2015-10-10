use std::cmp;

use libserver_types::*;


#[derive(Clone, Copy)]
pub struct Line {
    pub left: V2,
    pub base: i32,
}

impl Line {
    pub fn new(a: V2, b: V2) -> Line {
        let ab = b - a;
        let left = V2::new(-ab.y, ab.x);
        Line {
            left: left,
            base: left.dot(a),
        }
    }

    /// Check where a point sits relative to the line.  Returns a positive number if the point is
    /// to the left of the line (from `a` through `b`), negative if to the right, and zero if the
    /// point is exactly on the line.
    pub fn delta(&self, p: V2) -> i32 {
        self.left.dot(p) - self.base
    }
}

#[derive(Clone)]
pub struct Triangle {
    pub bounds: Region<V2>,

    pub a: Line,
    pub b: Line,
    pub c: Line,
}

impl Triangle {
    pub fn new(a: V2, b: V2, c: V2) -> Triangle {
        let min_x = cmp::min(cmp::min(a.x, b.x), c.x);
        let min_y = cmp::min(cmp::min(a.y, b.y), c.y);
        let max_x = cmp::max(cmp::max(a.x, b.x), c.x);
        let max_y = cmp::max(cmp::max(a.y, b.y), c.y);

        let ab = b - a;
        let ac = c - a;
        let left = V2::new(-ab.y, ab.x);
        let (a, b, c) =
            if ac.dot(left) < 0 {
                (a, c, b)
            } else {
                (a, b, c)
            };

        Triangle {
            bounds: Region::new(V2::new(min_x, min_y), V2::new(max_x, max_y)),

            a: Line::new(a, b),
            b: Line::new(b, c),
            c: Line::new(c, a),
        }
    }

    /// Check whether a point is inside the triangle.
    pub fn contains(&self, p: V2) -> bool {
        // Assumes counterclockwise winding for the triangle.
        self.a.delta(p) >= 0 &&
        self.b.delta(p) >= 0 &&
        self.c.delta(p) >= 0
    }
}

