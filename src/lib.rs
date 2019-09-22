use std::collections::HashMap;

mod geom;
mod render;

pub use crate::geom::*;
use crate::render::*;

#[derive(Copy, Clone)]
pub struct PathId(usize);

#[derive(Copy, Clone)]
pub struct FontId(usize);

pub struct Scene {
    renderer: Renderer,
    width: f32,
    height: f32,

    paths: Vec<Path>,
    indices_free: u16,
    vertices_free: u16,
    fonts: Vec<Font>,
    vertices: Vec<Vertex>,
    indices: Vec<u16>,
}

impl Scene {
    pub fn new(width: f32, height: f32) -> Scene {
        Scene {
            renderer: Renderer::new(),
            width,
            height,

            paths: Vec::new(),
            indices_free: 0,
            vertices_free: 0,
            fonts: Vec::new(),
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    pub fn set_size(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;
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

    pub fn draw_path(&mut self, path: PathId, x: f32, y: f32, color: Color) {
        self.draw_path_transformed(path, x, y, color, Mat2x2::id());
    }

    pub fn draw_path_transformed(&mut self, path: PathId, x: f32, y: f32, color: Color, transform: Mat2x2) {
        let path = &self.paths[path.0];

        let p = Vec2::new(x, y) + transform * Vec2::new(path.offset.x, path.offset.y);
        let v1 = transform * Vec2::new(path.size.x, 0.0);
        let v2 = transform * Vec2::new(0.0, path.size.y);
        let n1 = v1.normalized();
        let n2 = v2.normalized();

        let d = 0.5 / n1.cross(n2).abs();
        let d1 = d * n1;
        let d2 = d * n2;

        let dx = d / v1.length();
        let dy = d / v2.length();

        let p1 = (p - d1 - d2).pixel_to_ndc(self.width, self.height);
        let p2 = (p + v1 + d1 - d2).pixel_to_ndc(self.width, self.height);
        let p3 = (p + v1 + d1 + v2 + d2).pixel_to_ndc(self.width, self.height);
        let p4 = (p - d1 + v2 + d2).pixel_to_ndc(self.width, self.height);

        let col = color.to_linear_premul();

        let i = self.vertices.len() as u16;
        self.vertices.extend_from_slice(&[
            Vertex { pos: [p1.x, p1.y], col, uv: [-dx, -dy], path: [path.start, path.end] },
            Vertex { pos: [p2.x, p2.y], col, uv: [1.0 + dx, -dy], path: [path.start, path.end] },
            Vertex { pos: [p3.x, p3.y], col, uv: [1.0 + dx, 1.0 + dy], path: [path.start, path.end] },
            Vertex { pos: [p4.x, p4.y], col, uv: [-dx, 1.0 + dy], path: [path.start, path.end] },
        ]);
        self.indices.extend_from_slice(&[i, i + 1, i + 2, i, i + 2, i + 3]);
    }

    pub fn draw_text(&mut self, font_id: FontId, size: f32, text: &str, x: f32, y: f32, color: Color) {
        self.draw_text_transformed(font_id, size, text, x, y, color, Mat2x2::id());
    }

    pub fn draw_text_transformed(&mut self, font_id: FontId, size: f32, text: &str, x: f32, y: f32, color: Color, transform: Mat2x2) {
        let (units_per_em, ascender) = {
            let font = &self.fonts[font_id.0].font;
            (font.units_per_em().unwrap() as f32, font.ascender() as f32)
        };
        let scale = size / units_per_em;
        let mut pos = Vec2::new(x, y + scale * ascender);
        for c in text.chars() {
            if let Ok(glyph_id) = self.fonts[font_id.0].font.glyph_index(c) {
                let transformed = transform * pos;
                if let Some(&path) = self.fonts[font_id.0].glyphs.get(&glyph_id) {
                    self.draw_path_transformed(path, transformed.x, transformed.y, color, scale * transform);
                } else if let Ok(glyph) = self.fonts[font_id.0].font.glyph(glyph_id) {
                    let path = Self::build_glyph(&glyph).build(self);
                    self.fonts[font_id.0].glyphs.insert(glyph_id, path);
                    self.draw_path_transformed(path, transformed.x, transformed.y, color, scale * transform);
                };

                pos.x += scale * self.fonts[font_id.0].font.glyph_hor_metrics(glyph_id).unwrap().advance as f32;
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
    offset: Vec2,
    size: Vec2,
    start: u16,
    end: u16,
}

pub struct PathBuilder {
    vertices: Vec<Vec2>,
    indices: Vec<usize>,
    first: Vec2,
    last: Vec2,
}

impl PathBuilder {
    pub fn new() -> PathBuilder {
        PathBuilder {
            vertices: Vec::new(),
            indices: Vec::new(),
            first: Vec2::new(0.0, 0.0),
            last: Vec2::new(0.0, 0.0),
        }
    }

    pub fn move_to(&mut self, x: f32, y: f32) -> &mut Self {
        self.close();
        self.first = Vec2::new(x, y);
        self.last = Vec2::new(x, y);
        self
    }

    pub fn line_to(&mut self, x: f32, y: f32) -> &mut Self {
        let point = Vec2::new(x, y);

        if self.last.y == point.y {
            self.vertices.push(self.last);
            self.vertices.push(Vec2::new(0.0, 0.0));
            self.last = point;
            return self;
        }

        self.indices.push(self.vertices.len());

        self.vertices.push(self.last);
        self.vertices.push(0.5 * (self.last + point));
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

        fn split_at(p1: Vec2, p2: Vec2, p3: Vec2, t: f32) -> (Vec2, Vec2, Vec2, Vec2, Vec2) {
            let p12 = Vec2::lerp(t, p1, p2);
            let p23 = Vec2::lerp(t, p2, p3);
            let point = Vec2::lerp(t, p12, p23);
            (p1, p12, point, p23, p3)
        }

        let p1 = self.last;
        let p2 = Vec2::new(x1, y1);
        let p3 = Vec2::new(x2, y2);

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

                let i = self.vertices.len();
                self.indices.extend_from_slice(&[i, i + 2, i + 4]);
                self.vertices.extend_from_slice(&[p1, p2, p3, p4, p5, p6]);
                self.last = p7;
            }
            (Some(split), None) | (None, Some(split)) => {
                let (p1, p2, p3, p4, p5) = split_at(p1, p2, p3, split);

                let i = self.vertices.len();
                self.indices.extend_from_slice(&[i, i + 2]);
                self.vertices.extend_from_slice(&[p1, p2, p3, p4]);
                self.last = p5;
            }
            (None, None) => {
                self.indices.push(self.vertices.len());
                self.vertices.extend_from_slice(&[p1, p2]);
                self.last = p3;
            }
        }

        self.last = Vec2::new(x2, y2);

        self
    }

    pub fn arc_to(&mut self, radius: f32, large_arc: bool, winding: bool, x: f32, y: f32) -> &mut Self {
        const MAX_ANGLE: f32 = std::f32::consts::PI / 4.0;

        let end = Vec2::new(x, y);
        let to_midpoint = 0.5 * (end - self.last);
        let to_midpoint_len = to_midpoint.length();
        let radius = radius.max(to_midpoint_len);
        let to_center_len = (radius * radius - to_midpoint_len * to_midpoint_len).sqrt();
        let center_dir = if large_arc == winding { -1.0 } else { 1.0 };
        let to_center = if to_midpoint.length() == 0.0 {
            Vec2::new(-1.0, 0.0)
        } else {
            Vec2::new(-to_midpoint.y, to_midpoint.x).normalized()
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
            let normal = Vec2::new(angle.cos(), angle.sin());
            let point = center + radius * normal;

            let tangent = Vec2::new(-normal.y, normal.x);
            let control = point + 0.5 * ((self.last - point).length() / tangent.dot((self.last - point).normalized())) * tangent;

            self.quadratic_to(control.x, control.y, point.x, point.y);
        }

        self
    }

    fn close(&mut self) {
        if self.first.distance(self.last) > 1.0e-6 {
            self.line_to(self.first.x, self.first.y);
        }

        self.vertices.push(self.last);
        self.vertices.push(Vec2::new(0.0, 0.0));
    }

    pub fn build(&mut self, graphics: &mut Scene) -> PathId {
        self.close();

        let (mut min_x, mut max_x) = (std::f32::INFINITY, -std::f32::INFINITY);
        let (mut min_y, mut max_y) = (std::f32::INFINITY, -std::f32::INFINITY);
        for i in self.indices.iter() {
            for p in &self.vertices[*i .. *i + 3] {
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

        let offset = Vec2::new(min_x, min_y);
        let size = Vec2::new(max_x - min_x, max_y - min_y);

        let indices: Vec<u16> = self.indices.iter().map(|i| (*i as u16 + graphics.vertices_free as u16 / 2) / 2).collect();
        graphics.renderer.upload_indices(graphics.indices_free as u16, &indices);

        let mut vertices = Vec::with_capacity(self.vertices.len() * 2);
        for point in self.vertices.iter() {
            vertices.extend_from_slice(&[
                (((point.x - min_x) / size.x) * std::u16::MAX as f32).round() as u16,
                (((point.y - min_y) / size.y) * std::u16::MAX as f32).round() as u16,
            ]);
        }
        graphics.renderer.upload_vertices(graphics.vertices_free as u16, &vertices);

        let id = graphics.paths.len();
        graphics.paths.push(Path {
            offset,
            size,
            start: graphics.indices_free,
            end: graphics.indices_free + self.indices.len() as u16,
        });
        graphics.indices_free += indices.len() as u16;
        graphics.vertices_free += vertices.len() as u16;

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
