use sdl2::rect::Point;


use crate::RESOLUTION;


pub(crate) fn line_intersection(y: i32, p0: &Point, p1: &Point) -> Option<i32> {
    if (p0.y > y && p1.y > y) || (p0.y < y && p1.y < y) { return None }
    let p0x = p0.x as f32;
    let p1x = p1.x as f32;
    let p2x = RESOLUTION.0 as f32;
    let p3x = RESOLUTION.1 as f32;
    let p0y = p0.y as f32;
    let p1y = p1.y as f32;
    let p2y = y as f32;
    let p3y = y as f32;
    let t: f32 = 
        ((p0x-p2x)*(p2y-p3y)-(p0y-p2y)*(p2x-p3x)) /
        ((p0x-p1x)*(p2y-p3y)-(p0y-p1y)*(p2x-p3x));
    let x = (p0x + t*(p1x-p0x)).ceil();

    if x.is_normal() {
        Some(x as i32)
    } else {
        None
    }
}
