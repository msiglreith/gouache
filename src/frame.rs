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

    pub fn draw_path(&mut self, path: &Path, position: Vec2, transform: Mat2x2, color: Color) {
        let entry = if let Some(entry) = self.cache.get_path(path.key) {
            entry
        } else {
            let entry = self.cache.place_path(path.key, path);

            let indices: Vec<u16> = path.indices.iter().map(|index| entry.vertices / 4 + *index as u16 / 2).collect();
            self.renderer.upload_indices(entry.indices, &indices);

            let mut vertices = Vec::with_capacity(path.vertices.len() * 2);
            for vertex in path.vertices.iter() {
                vertices.extend_from_slice(&[
                    (((vertex.x - path.offset.x) / path.size.x) * std::u16::MAX as f32).round() as u16,
                    (((vertex.y - path.offset.y) / path.size.y) * std::u16::MAX as f32).round() as u16,
                ]);
            }
            self.renderer.upload_vertices(entry.vertices, &vertices);

            entry
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
        let path = [entry.indices, entry.indices + path.indices.len() as u16];

        let i = self.vertices.len() as u16;
        self.vertices.extend_from_slice(&[
            Vertex { pos: [positions[0].x, positions[0].y], col, uv: [-dx, -dy], path },
            Vertex { pos: [positions[1].x, positions[1].y], col, uv: [1.0 + dx, -dy], path },
            Vertex { pos: [positions[2].x, positions[2].y], col, uv: [1.0 + dx, 1.0 + dy], path },
            Vertex { pos: [positions[3].x, positions[3].y], col, uv: [-dx, 1.0 + dy], path },
        ]);
        self.indices.extend_from_slice(&[i, i + 1, i + 2, i, i + 2, i + 3]);
    }

    pub fn draw_text(&mut self, font: &mut Font, text: &TextLayout, position: Vec2, transform: Mat2x2, color: Color) {
        let scaled_transform = text.scale * transform;
        for glyph in text.glyphs.iter() {
            let path = font.get_glyph(glyph.glyph_key, self.cache);
            if path.indices.len() > 0 {
                self.draw_path(path, position + transform * glyph.position, scaled_transform, color);            
            }
        }
    }

    pub fn finish(self) {
        self.renderer.draw(&self.vertices, &self.indices);
    }
}


#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct PathKey(u32);

#[derive(Copy, Clone)]
pub struct PathEntry {
    pub indices: u16,
    pub vertices: u16,
}

pub struct Cache {
    paths: HashMap<PathKey, PathEntry>,
    next_path_key: u32,
    next_font_key: u32,
    indices_free: u16,
    vertices_free: u16,
}

impl Cache {
    pub fn new() -> Cache {
        Cache {
            paths: HashMap::new(),
            next_path_key: 0,
            next_font_key: 0,
            indices_free: 0,
            vertices_free: 0
        }
    }

    pub fn add_path(&mut self) -> PathKey {
        let path_key = self.next_path_key;
        self.next_path_key += 1;
        PathKey(path_key)
    }

    pub fn get_path(&self, key: PathKey) -> Option<PathEntry> {
        self.paths.get(&key).map(|entry| *entry)
    }

    pub fn place_path(&mut self, key: PathKey, path: &Path) -> PathEntry {
        let entry = PathEntry {
            indices: self.indices_free,
            vertices: self.vertices_free,
        };
        self.indices_free += path.indices.len() as u16;
        self.vertices_free += 2 * path.vertices.len() as u16;
        self.paths.insert(key, entry);
        entry
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
