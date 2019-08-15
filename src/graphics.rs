use crate::render::*;

#[derive(Copy, Clone)]
pub struct PathId(usize);

pub struct Graphics {
    renderer: Renderer,
    width: f32,
    height: f32,
    paths: Vec<Path>,
    free_path: u16,
    vertices: Vec<Vertex>,
    indices: Vec<u16>,
}

impl Graphics {
    pub fn new(width: f32, height: f32) -> Graphics {
        Graphics {
            renderer: Renderer::new(),
            width,
            height,
            paths: Vec::new(),
            free_path: 0,
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    pub fn set_size(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;
    }

    pub fn clear(&mut self, color: Color) {
        self.renderer.clear(color.to_linear_premul());
    }

    pub fn begin_frame(&mut self) {
        self.vertices = Vec::new();
        self.indices = Vec::new();
    }

    pub fn end_frame(&mut self) {
        self.renderer.draw(&self.vertices, &self.indices);
    }

    pub fn draw_path(&mut self, x: f32, y: f32, scale: f32, path: PathId) {
        let path = &self.paths[path.0];
        let min = (Point::new(x, y) + scale * path.offset - Point::new(1.0, 1.0))
            .pixel_to_ndc(self.width, self.height);
        let max = (Point::new(x, y) + scale * (path.offset + path.size) + Point::new(1.0, 1.0))
            .pixel_to_ndc(self.width, self.height);
        let dx = 1.0 / (scale * path.size.x as f32);
        let dy = 1.0 / (scale * path.size.y as f32);
        let i = self.vertices.len() as u16;
        self.vertices.extend_from_slice(&[
            Vertex { pos: [min.x, min.y], uv: [-dx, -dy], path: [path.start, path.length] },
            Vertex { pos: [max.x, min.y], uv: [1.0 + dx, -dy], path: [path.start, path.length] },
            Vertex { pos: [max.x, max.y], uv: [1.0 + dx, 1.0 + dy], path: [path.start, path.length] },
            Vertex { pos: [min.x, max.y], uv: [-dx, 1.0 + dy], path: [path.start, path.length] },
        ]);
        self.indices.extend_from_slice(&[i, i + 1, i + 2, i, i + 2, i + 3]);
    }
}

struct Path {
    offset: Point,
    size: Point,
    start: u16,
    length: u16,
}

struct Segment {
    p1: Point,
    p2: Point,
    p3: Point,
}

impl Segment {
    fn split(&self, t: f32) -> (Segment, Segment) {
        let p12 = Point::lerp(t, self.p1, self.p2);
        let p23 = Point::lerp(t, self.p2, self.p3);
        let point = Point::lerp(t, p12, p23);
        (Segment { p1: self.p1, p2: p12, p3: point }, Segment { p1: point, p2: p23, p3: self.p3 })
    }
}

pub struct PathBuilder {
    segments: Vec<Segment>,
    first: Point,
    last: Point,
}

impl PathBuilder {
    pub fn new() -> PathBuilder {
        PathBuilder {
            segments: Vec::new(),
            first: Point::new(0.0, 0.0),
            last: Point::new(0.0, 0.0),
        }
    }

    pub fn move_to(&mut self, x: f32, y: f32) -> &mut Self {
        self.close();
        self.first = Point::new(x, y);
        self.last = Point::new(x, y);
        self
    }

    pub fn line_to(&mut self, x: f32, y: f32) -> &mut Self {
        let point = Point::new(x, y);

        self.segments.push(Segment {
            p1: self.last,
            p2: 0.5 * (self.last + point),
            p3: point,
        });

        self.last = point;

        self
    }

    pub fn quadratic_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32) -> &mut Self {
        fn monotone(a: f32, b: f32, c: f32) -> bool {
            (a <= b && b <= c) || (c <= b && b <= a)
        }

        fn solve(a: f32, b: f32, c: f32) -> f32 {
            (a - b) / (a - 2.0 * b + c)
        }

        let segment = Segment { p1: self.last, p2: Point::new(x1, y1), p3: Point::new(x2, y2) };

        let x_split = if !monotone(segment.p1.x, segment.p2.x, segment.p3.x) {
            Some(solve(segment.p1.x, segment.p2.x, segment.p3.x))
        } else {
            None
        };
        let y_split = if !monotone(segment.p1.y, segment.p2.y, segment.p3.y) {
            Some(solve(segment.p1.y, segment.p2.y, segment.p3.y))
        } else {
            None
        };

        match (x_split, y_split) {
            (Some(x_split), Some(y_split)) => {
                let (split1, split2) = (x_split.min(y_split), x_split.max(y_split));
                let (first, rest) = segment.split(split1);
                let (second, third) = rest.split((split2 - split1) / (1.0 - split1));
                self.segments.push(first);
                self.segments.push(second);
                self.segments.push(third);
            }
            (Some(split), None) | (None, Some(split)) => {
                let (first, second) = segment.split(split);
                self.segments.push(first);
                self.segments.push(second);
            }
            (None, None) => {
                self.segments.push(segment);
            }
        }

        self.last = Point::new(x2, y2);

        self
    }

    fn close(&mut self) {
        if self.first.distance(self.last) > 1.0e-6 {
            self.line_to(self.first.x, self.first.y);
        }
    }

    pub fn build(&mut self, graphics: &mut Graphics) -> PathId {
        self.close();

        let (mut min_x, mut max_x) = (std::f32::INFINITY, -std::f32::INFINITY);
        let (mut min_y, mut max_y) = (std::f32::INFINITY, -std::f32::INFINITY);
        for segment in self.segments.iter() {
            for p in &[segment.p1, segment.p2, segment.p3] {
                min_x = min_x.min(p.x);
                max_x = max_x.max(p.x);
                min_y = min_y.min(p.y);
                max_y = max_y.max(p.y);
            }
        }
        if !min_x.is_finite() { min_x = 0.0; }
        if !max_x.is_finite() { max_x = 0.0; }
        if !min_y.is_finite() { min_y = 0.0; }
        if !max_y.is_finite() { max_y = 0.0; }

        let offset = Point::new(min_x, min_y);
        let size = Point::new(max_x - min_x, max_y - min_y);

        let mut data = Vec::with_capacity(self.segments.len() * 6);
        for segment in self.segments.iter() {
            data.extend([
                (segment.p1.x - min_x) / size.x, (segment.p1.y - min_y) / size.y,
                (segment.p2.x - min_x) / size.x, (segment.p2.y - min_y) / size.y,
                (segment.p3.x - min_x) / size.x, (segment.p3.y - min_y) / size.y,
            ].iter().map(|x| (x * std::u16::MAX as f32).round() as u16));
        }

        graphics.renderer.upload_paths(graphics.free_path as u16, &data);

        let id = graphics.paths.len();
        graphics.paths.push(Path {
            offset,
            size,
            start: graphics.free_path,
            length: graphics.free_path + self.segments.len() as u16,
        });
        graphics.free_path += self.segments.len() as u16;

        PathId(id)
    }
}

#[derive(Copy, Clone)]
pub struct Color {
    pub r: f32, pub g: f32, pub b: f32, pub a: f32,
}

impl Color {
    pub fn rgba(r: f32, g: f32, b: f32, a: f32) -> Color {
        Color { r, g, b, a }
    }

    fn to_linear_premul(&self) -> [f32; 4] {
        [
            self.a * srgb_to_linear(self.r),
            self.a * srgb_to_linear(self.g),
            self.a * srgb_to_linear(self.b),
            self.a
        ]
    }
}

fn srgb_to_linear(x: f32) -> f32 {
    if x < 0.04045 { x / 12.92 } else { ((x + 0.055) / 1.055).powf(2.4)  }
}

use std::ops;

#[derive(Copy, Clone, Debug)]
struct Point { x: f32, y: f32 }

impl Point {
    #[inline]
    pub fn new(x: f32, y: f32) -> Point {
        Point { x: x, y: y }
    }

    #[inline]
    pub fn distance(self, other: Point) -> f32 {
        ((other.x - self.x) * (other.x - self.x) + (other.y - self.y) * (other.y - self.y)).sqrt()
    }

    #[inline]
    pub fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    #[inline]
    pub fn normalized(self) -> Point {
        let len = self.length();
        Point { x: self.x / len, y: self.y / len }
    }

    #[inline]
    pub fn lerp(t: f32, a: Point, b: Point) -> Point {
        (1.0 - t) * a + t * b
    }

    #[inline]
    fn pixel_to_ndc(self, screen_width: f32, screen_height: f32) -> Point {
        Point {
            x: 2.0 * (self.x / screen_width as f32 - 0.5),
            y: 2.0 * (1.0 - self.y / screen_height as f32 - 0.5),
        }
    }
}

impl ops::Add for Point {
    type Output = Point;
    #[inline]
    fn add(self, rhs: Point) -> Point {
        Point { x: self.x + rhs.x, y: self.y + rhs.y }
    }
}

impl ops::AddAssign for Point {
    #[inline]
    fn add_assign(&mut self, other: Point) {
        *self = *self + other;
    }
}

impl ops::Sub for Point {
    type Output = Point;
    #[inline]
    fn sub(self, rhs: Point) -> Point {
        Point { x: self.x - rhs.x, y: self.y - rhs.y }
    }
}

impl ops::SubAssign for Point {
    #[inline]
    fn sub_assign(&mut self, other: Point) {
        *self = *self - other;
    }
}

impl ops::Mul<f32> for Point {
    type Output = Point;
    #[inline]
    fn mul(self, rhs: f32) -> Point {
        Point { x: self.x * rhs, y: self.y * rhs }
    }
}

impl ops::Mul<Point> for f32 {
    type Output = Point;
    #[inline]
    fn mul(self, rhs: Point) -> Point {
        Point { x: self * rhs.x, y: self * rhs.y }
    }
}

impl ops::MulAssign<f32> for Point {
    #[inline]
    fn mul_assign(&mut self, other: f32) {
        *self = *self * other;
    }
}
