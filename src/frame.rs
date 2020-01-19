use crate::geom::*;
use crate::path::*;
use crate::renderer::*;
use crate::font::*;

use std::collections::HashMap;

pub struct Frame<'c, 'r> {
    cache: &'c mut Cache,
    renderer: &'r mut dyn Renderer,
    width: f32,
    height: f32,
    vertices: Vec<Vertex>,
    indices: Vec<u16>,
}

impl<'c, 'r> Frame<'c, 'r> {
    pub fn new(cache: &'c mut Cache, renderer: &'r mut dyn Renderer, width: f32, height: f32) -> Frame<'c, 'r> {
        Frame {
            cache,
            renderer,
            width,
            height,
            vertices: Vec::new(),
            indices: Vec::new(),
        }
    }

    pub fn clear(&mut self, color: Color) {
        self.renderer.clear(color.to_linear_premul());
    }

    pub fn draw_path(&mut self, path: &Path, path_key: PathKey, position: Vec2, transform: Mat2x2, color: Color) {
        let index = if let Some(&index) = self.cache.paths.get(&path_key) {
            index
        } else {
            let index = self.cache.paths_free;
            self.cache.paths_free += path.buffer.len() as u16;
            self.cache.paths.insert(path_key, index);

            self.renderer.upload(index, &path.buffer);

            index
        };

        let p = position + transform * Vec2::new(path.offset.x, path.offset.y);
        let v1 = transform * Vec2::new(path.size.x, 0.0);
        let v2 = transform * Vec2::new(0.0, path.size.y);
        let n1 = v1.normalized();
        let n2 = v2.normalized();

        let d = 0.5 / n1.cross(n2).abs();
        let d1 = d * n1;
        let d2 = d * n2;

        let dx = d / v1.length();
        let dy = d / v2.length();

        let positions = [
            (p - d1 - d2).pixel_to_ndc(self.width, self.height),
            (p + v1 + d1 - d2).pixel_to_ndc(self.width, self.height),
            (p + v1 + d1 + v2 + d2).pixel_to_ndc(self.width, self.height),
            (p - d1 + v2 + d2).pixel_to_ndc(self.width, self.height),
        ];
        let col = color.to_linear_premul();
        let path = [index, index + path.buffer.len() as u16];

        let i = self.vertices.len() as u16;
        self.vertices.extend_from_slice(&[
            Vertex { pos: [positions[0].x, positions[0].y], col, uv: [-dx, -dy], path },
            Vertex { pos: [positions[1].x, positions[1].y], col, uv: [1.0 + dx, -dy], path },
            Vertex { pos: [positions[2].x, positions[2].y], col, uv: [1.0 + dx, 1.0 + dy], path },
            Vertex { pos: [positions[3].x, positions[3].y], col, uv: [-dx, 1.0 + dy], path },
        ]);
        self.indices.extend_from_slice(&[i, i + 1, i + 2, i, i + 2, i + 3]);
    }

    pub fn draw_text(&mut self, font: &Font, font_key: FontKey, size: f32, text: &str, position: Vec2, transform: Mat2x2, color: Color) {
        let mut glyphs = std::mem::replace(&mut self.cache.glyphs, HashMap::new());
        for glyph in font.layout(text, size) {
            let key = (font_key, glyph.glyph_key);
            let entry = if let Some(entry) = glyphs.get(&key) {
                entry
            } else {
                glyphs.insert(key, GlyphEntry {
                    path: font.build_glyph(glyph.glyph_key),
                    key: self.cache.add_path(),
                });
                glyphs.get(&key).unwrap()
            };

            if entry.path.buffer.len() > 0 {
                self.draw_path(&entry.path, entry.key, position + transform * glyph.position, glyph.scale * transform, color);
            }
        }
        self.cache.glyphs = glyphs;
    }

    pub fn draw_rect(&mut self, position: Vec2, dimensions: Vec2, transform: Mat2x2, color: Color) {
        if self.cache.rect.is_none() {
            let path = PathBuilder::new()
                .line_to(0.0, 1.0)
                .line_to(1.0, 1.0)
                .line_to(1.0, 0.0)
                .build();
            self.cache.rect = Some((self.cache.add_path(), path));
        }

        let (path_key, path) = self.cache.rect.take().unwrap();
        self.draw_path(&path, path_key, position, transform * Mat2x2::new(dimensions.x, 0.0, 0.0, dimensions.y), color);
        self.cache.rect = Some((path_key, path));
    }

    pub fn finish(self) {
        self.renderer.draw(&self.vertices, &self.indices);
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct PathKey(u32);

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct FontKey(u32);

#[derive(Copy, Clone)]
struct PathEntry {
    indices: u16,
    vertices: u16,
}

struct GlyphEntry {
    path: Path,
    key: PathKey,
}

pub struct Cache {
    paths: HashMap<PathKey, u16>,
    next_path_key: u32,
    glyphs: HashMap<(FontKey, GlyphKey), GlyphEntry>,
    next_font_key: u32,
    rect: Option<(PathKey, Path)>,
    paths_free: u16,
}

impl Cache {
    pub fn new() -> Cache {
        Cache {
            paths: HashMap::new(),
            next_path_key: 1,
            glyphs: HashMap::new(),
            next_font_key: 1,
            rect: None,
            paths_free: 0,
        }
    }

    pub fn add_path(&mut self) -> PathKey {
        let path_key = self.next_path_key;
        self.next_path_key += 1;
        PathKey(path_key)
    }

    pub fn add_font(&mut self) -> FontKey {
        let font_key = self.next_font_key;
        self.next_font_key += 1;
        FontKey(font_key)
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
        fn srgb_to_linear(x: f32) -> f32 {
            if x < 0.04045 { x / 12.92 } else { ((x + 0.055) / 1.055).powf(2.4)  }
        }

        [
            self.a * srgb_to_linear(self.r),
            self.a * srgb_to_linear(self.g),
            self.a * srgb_to_linear(self.b),
            self.a
        ]
    }
}
