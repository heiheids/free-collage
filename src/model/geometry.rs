use tiny_skia as sk;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    pub fn distance(self, other: Self) -> f32 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }

    pub fn lerp(self, other: Self, t: f32) -> Self {
        Self::new(
            self.x + (other.x - self.x) * t,
            self.y + (other.y - self.y) * t,
        )
    }

    pub fn round(self) -> Self {
        Self::new(self.x.round(), self.y.round())
    }
}

impl std::ops::Add for Point {
    type Output = Point;
    fn add(self, rhs: Point) -> Point {
        Point::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl std::ops::Sub for Point {
    type Output = Point;
    fn sub(self, rhs: Point) -> Point {
        Point::new(self.x - rhs.x, self.y - rhs.y)
    }
}

impl std::ops::Mul<f32> for Point {
    type Output = Point;
    fn mul(self, rhs: f32) -> Point {
        Point::new(self.x * rhs, self.y * rhs)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Rect {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0, w: 0.0, h: 0.0 };

    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self { x, y, w, h }
    }

    pub fn from_xywh(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self::new(x, y, w.max(0.0), h.max(0.0))
    }

    pub fn from_points(a: Point, b: Point) -> Self {
        let x = a.x.min(b.x);
        let y = a.y.min(b.y);
        Self::new(x, y, (a.x - b.x).abs(), (a.y - b.y).abs())
    }

    pub fn from_ltrb(l: f32, t: f32, r: f32, b: f32) -> Self {
        Self::new(l, t, (r - l).max(0.0), (b - t).max(0.0))
    }

    pub fn right(self) -> f32 {
        self.x + self.w
    }

    pub fn bottom(self) -> f32 {
        self.y + self.h
    }

    pub fn center(self) -> Point {
        Point::new(self.x + self.w * 0.5, self.y + self.h * 0.5)
    }

    pub fn is_empty(self) -> bool {
        self.w <= 0.0 || self.h <= 0.0
    }

    pub fn contains(self, p: Point) -> bool {
        p.x >= self.x && p.x <= self.right() && p.y >= self.y && p.y <= self.bottom()
    }

    pub fn inflate(self, d: f32) -> Self {
        Self::new(self.x - d, self.y - d, self.w + d * 2.0, self.h + d * 2.0)
    }

    pub fn offset(self, dx: f32, dy: f32) -> Self {
        Self::new(self.x + dx, self.y + dy, self.w, self.h)
    }

    pub fn union(self, other: Self) -> Self {
        if self.is_empty() {
            return other;
        }
        if other.is_empty() {
            return self;
        }
        let l = self.x.min(other.x);
        let t = self.y.min(other.y);
        let r = self.right().max(other.right());
        let b = self.bottom().max(other.bottom());
        Self::from_ltrb(l, t, r, b)
    }

    pub fn intersect(self, other: Self) -> Self {
        let l = self.x.max(other.x);
        let t = self.y.max(other.y);
        let r = self.right().min(other.right());
        let b = self.bottom().min(other.bottom());
        if r <= l || b <= t {
            Self::ZERO
        } else {
            Self::from_ltrb(l, t, r, b)
        }
    }

    pub fn round(self) -> Self {
        Self::from_ltrb(
            self.x.floor(),
            self.y.floor(),
            self.right().ceil(),
            self.bottom().ceil(),
        )
    }

    pub fn to_skia(self) -> Option<sk::Rect> {
        sk::Rect::from_xywh(self.x, self.y, self.w.max(0.01), self.h.max(0.01))
    }
}

pub fn point_in_ellipse(p: Point, rect: Rect) -> bool {
    if rect.w <= 0.0 || rect.h <= 0.0 {
        return false;
    }
    let cx = rect.x + rect.w * 0.5;
    let cy = rect.y + rect.h * 0.5;
    let rx = rect.w * 0.5;
    let ry = rect.h * 0.5;
    let dx = (p.x - cx) / rx;
    let dy = (p.y - cy) / ry;
    dx * dx + dy * dy <= 1.0
}

pub fn dist_to_segment(p: Point, a: Point, b: Point) -> f32 {
    let abx = b.x - a.x;
    let aby = b.y - a.y;
    let len2 = abx * abx + aby * aby;
    if len2 <= f32::EPSILON {
        return p.distance(a);
    }
    let t = (((p.x - a.x) * abx + (p.y - a.y) * aby) / len2).clamp(0.0, 1.0);
    p.distance(Point::new(a.x + abx * t, a.y + aby * t))
}

pub fn distance_to_polyline(p: Point, points: &[Point]) -> f32 {
    if points.is_empty() {
        return f32::MAX;
    }
    if points.len() == 1 {
        return p.distance(points[0]);
    }
    points
        .windows(2)
        .map(|w| dist_to_segment(p, w[0], w[1]))
        .fold(f32::MAX, f32::min)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn union_and_intersect() {
        let a = Rect::new(10.0, 10.0, 20.0, 20.0);
        let b = Rect::new(20.0, 25.0, 40.0, 10.0);
        let u = a.union(b);
        assert_eq!((u.x, u.y, u.right(), u.bottom()), (10.0, 10.0, 60.0, 35.0));
        let i = a.intersect(b);
        assert_eq!((i.x, i.y, i.w, i.h), (20.0, 25.0, 10.0, 5.0));
    }

    #[test]
    fn segment_distance() {
        assert!((dist_to_segment(Point::new(5.0, 5.0), Point::ZERO, Point::new(10.0, 0.0)) - 5.0).abs() < 0.001);
    }
}
