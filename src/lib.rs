use std::collections::HashMap;

mod render;
use crate::render::*;

#[derive(Copy, Clone)]
pub struct PathId(usize);

#[derive(Copy, Clone)]
pub struct FontId(usize);

pub struct Graphics {
    renderer: Renderer,
    width: f32,
    height: f32,
    paths: Vec<Path>,
    indices_free: u16,
    points_free: u16,
    fonts: Vec<Font>,
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
            indices_free: 0,
            points_free: 0,
            fonts: Vec::new(),
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

    pub fn draw_path(&mut self, x: f32, y: f32, scale: f32, color: Color, path: PathId) {
        let path = &self.paths[path.0];

        let dilation = 0.5;
        let min = (Point::new(x, y) + scale * path.offset - Point::new(dilation, dilation))
            .pixel_to_ndc(self.width, self.height);
        let max = (Point::new(x, y) + scale * (path.offset + path.size) + Point::new(dilation, dilation))
            .pixel_to_ndc(self.width, self.height);
        let dx = dilation / (scale * path.size.x as f32);
        let dy = dilation / (scale * path.size.y as f32);

        let col = color.to_linear_premul();

        let i = self.vertices.len() as u16;
        self.vertices.extend_from_slice(&[
            Vertex { pos: [min.x, min.y], col, uv: [-dx, -dy], path: [path.start, path.end] },
            Vertex { pos: [max.x, min.y], col, uv: [1.0 + dx, -dy], path: [path.start, path.end] },
            Vertex { pos: [max.x, max.y], col, uv: [1.0 + dx, 1.0 + dy], path: [path.start, path.end] },
            Vertex { pos: [min.x, max.y], col, uv: [-dx, 1.0 + dy], path: [path.start, path.end] },
        ]);
        self.indices.extend_from_slice(&[i, i + 1, i + 2, i, i + 2, i + 3]);
    }

    pub fn add_font(&mut self, bytes: &'static [u8]) -> Option<FontId> {
        if let Ok(font) = ttf_parser::Font::from_data(bytes, 0) {
            let id = self.fonts.len();
            self.fonts.push(Font::new(font));
            Some(FontId(id))
        } else {
            None
        }
    }

    pub fn draw_text(&mut self, x: f32, y: f32, size: f32, font_id: FontId, color: Color, text: &str) {
        let (units_per_em, ascender) = {
            let font = &self.fonts[font_id.0].font;
            (font.units_per_em().unwrap() as f32, font.ascender() as f32)
        };
        let scale = size / units_per_em;
        let mut x = x;
        let y = y + scale * ascender;
        for c in text.chars() {
            if let Ok(glyph_id) = self.fonts[font_id.0].font.glyph_index(c) {
                if let Some(&path) = self.fonts[font_id.0].glyphs.get(&glyph_id) {
                    self.draw_path(x, y, scale, color, path);
                } else if let Ok(glyph) = self.fonts[font_id.0].font.glyph(glyph_id) {
                    let path = Self::build_glyph(&glyph).build(self);
                    self.fonts[font_id.0].glyphs.insert(glyph_id, path);
                    self.draw_path(x, y, scale, color, path);
                };

                x += scale * self.fonts[font_id.0].font.glyph_hor_metrics(glyph_id).unwrap().advance as f32;
            }
        }
    }

    fn build_glyph(glyph: &ttf_parser::glyf::Glyph) -> PathBuilder {
        use ttf_parser::glyf::OutlineBuilder;
        struct Builder { path: PathBuilder }
        impl OutlineBuilder for Builder {
            fn move_to(&mut self, x: f32, y: f32) {
                self.path.move_to(x, -y);
            }
            fn line_to(&mut self, x: f32, y: f32) {
                self.path.line_to(x, -y);
            }
            fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
                self.path.quadratic_to(x1, -y1, x, -y);
            }
            fn close(&mut self) {}
        }

        let mut builder = Builder { path: PathBuilder::new() };
        glyph.outline(&mut builder);
        builder.path
    }
}

struct Path {
    offset: Point,
    size: Point,
    start: u16,
    end: u16,
}

pub struct PathBuilder {
    points: Vec<Point>,
    indices: Vec<usize>,
    first: Point,
    last: Point,
}

impl PathBuilder {
    pub fn new() -> PathBuilder {
        PathBuilder {
            points: Vec::new(),
            indices: Vec::new(),
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

        self.indices.push(self.points.len());

        self.points.push(self.last);
        self.points.push(0.5 * (self.last + point));
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

        fn split_at(p1: Point, p2: Point, p3: Point, t: f32) -> (Point, Point, Point, Point, Point) {
            let p12 = Point::lerp(t, p1, p2);
            let p23 = Point::lerp(t, p2, p3);
            let point = Point::lerp(t, p12, p23);
            (p1, p12, point, p23, p3)
        }

        let p1 = self.last;
        let p2 = Point::new(x1, y1);
        let p3 = Point::new(x2, y2);

        let x_split = if !monotone(p1.x, p2.x, p3.x) {
            Some(solve(p1.x, p2.x, p3.x))
        } else {
            None
        };
        let y_split = if !monotone(p1.y, p2.y, p3.y) {
            Some(solve(p1.y, p2.y, p3.y))
        } else {
            None
        };

        match (x_split, y_split) {
            (Some(x_split), Some(y_split)) => {
                let (split1, split2) = (x_split.min(y_split), x_split.max(y_split));
                let (p1, p2, p3, p4, p5) = split_at(p1, p2, p3, split1);
                let (p3, p4, p5, p6, p7) = split_at(p3, p4, p5, (split2 - split1) / (1.0 - split1));

                let i = self.points.len();
                self.indices.extend_from_slice(&[i, i + 2, i + 4]);
                self.points.extend_from_slice(&[p1, p2, p3, p4, p5, p6]);
                self.last = p7;
            }
            (Some(split), None) | (None, Some(split)) => {
                let (p1, p2, p3, p4, p5) = split_at(p1, p2, p3, split);

                let i = self.points.len();
                self.indices.extend_from_slice(&[i, i + 2]);
                self.points.extend_from_slice(&[p1, p2, p3, p4]);
                self.last = p5;
            }
            (None, None) => {
                self.indices.push(self.points.len());
                self.points.extend_from_slice(&[p1, p2]);
                self.last = p3;
            }
        }

        self.last = Point::new(x2, y2);

        self
    }

    pub fn arc_to(&mut self, radius: f32, large_arc: bool, winding: bool, x: f32, y: f32) -> &mut Self {
        const MAX_ANGLE: f32 = std::f32::consts::PI / 4.0;

        let end = Point::new(x, y);
        let to_midpoint = 0.5 * (end - self.last);
        let to_midpoint_len = to_midpoint.length();
        let radius = radius.max(to_midpoint_len);
        let to_center_len = (radius * radius - to_midpoint_len * to_midpoint_len).sqrt();
        let center_dir = if large_arc == winding { -1.0 } else { 1.0 };
        let to_center = if to_midpoint.length() == 0.0 {
            Point::new(-1.0, 0.0)
        } else {
            Point::new(-to_midpoint.y, to_midpoint.x).normalized()
        };
        let center = self.last + to_midpoint + center_dir * to_center_len * to_center;

        let start_vector = self.last - center;
        let start_angle = start_vector.y.atan2(start_vector.x);
        let end_vector = end - center;
        let end_angle = {
            let end_angle = end_vector.y.atan2(end_vector.x);
            if winding && end_angle < start_angle {
                end_angle + 2.0 * std::f32::consts::PI
            } else if !winding && end_angle > start_angle {
                end_angle - 2.0 * std::f32::consts::PI
            } else {
                end_angle
            }
        };

        let num_segments = (((start_angle - end_angle).abs() / MAX_ANGLE).ceil() as usize).max(1).min(8);
        for i in 0..num_segments {
            let angle = start_angle + ((i + 1) as f32 / num_segments as f32) * (end_angle - start_angle);
            let normal = Point::new(angle.cos(), angle.sin());
            let point = center + radius * normal;

            let tangent = Point::new(-normal.y, normal.x);
            let control = point + 0.5 * ((self.last - point).length() / tangent.dot((self.last - point).normalized())) * tangent;

            self.quadratic_to(control.x, control.y, point.x, point.y);
        }

        self
    }

    fn close(&mut self) {
        if self.first.distance(self.last) > 1.0e-6 {
            self.line_to(self.first.x, self.first.y);
        }

        self.points.push(self.last);
        self.points.push(Point::new(0.0, 0.0));
    }

    pub fn build(&mut self, graphics: &mut Graphics) -> PathId {
        self.close();

        let (mut min_x, mut max_x) = (std::f32::INFINITY, -std::f32::INFINITY);
        let (mut min_y, mut max_y) = (std::f32::INFINITY, -std::f32::INFINITY);
        for i in self.indices.iter() {
            for p in &self.points[*i .. *i + 3] {
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

        let indices: Vec<u16> = self.indices.iter().map(|i| (*i as u16 + graphics.points_free as u16 / 2) / 2).collect();
        graphics.renderer.upload_indices(graphics.indices_free as u16, &indices);

        let mut points = Vec::with_capacity(self.points.len() * 2);
        for point in self.points.iter() {
            points.extend_from_slice(&[
                (((point.x - min_x) / size.x) * std::u16::MAX as f32).round() as u16,
                (((point.y - min_y) / size.y) * std::u16::MAX as f32).round() as u16,
            ]);
        }
        graphics.renderer.upload_points(graphics.points_free as u16, &points);

        let id = graphics.paths.len();
        graphics.paths.push(Path {
            offset,
            size,
            start: graphics.indices_free,
            end: graphics.indices_free + self.indices.len() as u16,
        });
        graphics.indices_free += indices.len() as u16;
        graphics.points_free += points.len() as u16;

        PathId(id)
    }
}

struct Font {
    font: ttf_parser::Font<'static>,
    glyphs: HashMap<u16, PathId>,
}

impl Font {
    fn new(font: ttf_parser::Font<'static>) -> Font {
        Font { font, glyphs: HashMap::new() }
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
    pub fn dot(self, other: Point) -> f32 {
        self.x * other.x + self.y * other.y
    }

    #[inline]
    pub fn distance(self, other: Point) -> f32 {
        (other - self).length()
    }

    #[inline]
    pub fn length(self) -> f32 {
        self.dot(self).sqrt()
    }

    #[inline]
    pub fn normalized(self) -> Point {
        (1.0 / self.length()) * self
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
