/// Simple geometry helpers used by the UI and layout code.
/// These are minimal placeholders; more advanced functionality can be added later.

/// 2‑D point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

/// Axis‑aligned rectangle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub origin: Point,
    pub size: Size,
}

impl Rect {
    pub fn new(origin: Point, size: Size) -> Self {
        Self { origin, size }
    }
    pub fn contains(&self, p: Point) -> bool {
        let (ox, oy) = (self.origin.x, self.origin.y);
        let (w, h) = (self.size.width, self.size.height);
        p.x >= ox && p.x <= ox + w && p.y >= oy && p.y <= oy + h
    }
}

/// Width and height.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub const ZERO: Self = Self {
        width: 0.0,
        height: 0.0,
    };
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}
