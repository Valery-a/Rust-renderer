use sdl2::rect::Point;
use std::cmp;

use crate::line_intersection;

#[derive(Debug, PartialEq)]
pub(crate) struct Face {
    a: Point,
    b: Point,
    c: Point
}

impl Face {
    pub fn new(a: Point, b: Point, c: Point) -> Face {
        Face { a, b, c }
    }

    pub fn orientation(&self) -> bool {
        let e0 = (self.b.x-self.a.x)*(self.b.y+self.a.y);
        let e1 = (self.c.x-self.b.x)*(self.c.y+self.b.y);
        let e2 = (self.a.x-self.c.x)*(self.a.y+self.c.y);
        e0+e1+e2 < 0
    }

    pub fn row_intersects(&self, y: i32) -> Option<(i32, i32)> {
        let (i0, i1, i2) = (
            line_intersection(y, &self.a, &self.b), 
            line_intersection(y, &self.b, &self.c), 
            line_intersection(y, &self.c, &self.a)
        );
    
        match (i0, i1, i2) {
            (Some(x0), Some(x1), None) => Some((cmp::min(x0, x1), cmp::max(x0, x1))),
            (Some(x0), None, Some(x1)) => Some((cmp::min(x0, x1), cmp::max(x0, x1))),
            (None, Some(x0), Some(x1)) => Some((cmp::min(x0, x1), cmp::max(x0, x1))),
            (Some(x0), Some(x1), Some(x2)) => Some((cmp::min(x0, x1), cmp::max(cmp::max(x1, x2), x0))),
            _ => None
        }
    }
    
    pub fn height_range(&self) -> (i32, i32) {
        (
            cmp::min(self.a.y, cmp::min(self.b.y, self.c.y)),
            cmp::max(self.a.y, cmp::max(self.b.y, self.c.y))
        )
    }

    pub fn barycentric(&self, p: &Point) -> (f32, f32, f32) {
        let vx0 = (self.b.x - self.a.x) as f32;
        let vy0 = (self.b.y - self.a.y) as f32;
        let vx1 = (self.c.x - self.a.x) as f32;
        let vy1 = (self.c.y - self.a.y) as f32;
        let vx2 = (     p.x - self.a.x) as f32;
        let vy2 = (     p.y - self.a.y) as f32;
        let den = vx0 * vy1 - vx1 * vy0;
        let v = (vx2 * vy1 - vx1 * vy2) / den;
        let w = (vx0 * vy2 - vx2 * vy0) / den;
        let u = 1. - v - w;
        (u, v, w)
    }
}
