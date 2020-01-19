use crate::geom::*;
use crate::path::*;
use crate::frame::Cache;

use std::collections::HashMap;
pub use ttf_parser::Error as FontError;
use ttf_parser::GlyphId;

pub struct Font<'a> {
    font: ttf_parser::Font<'a>,
    glyphs: HashMap<GlyphKey, Path>,
}

#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct GlyphKey(u16);

impl<'a> Font<'a> {
    pub fn from_bytes(bytes: &'a [u8]) -> Result<Font<'a>, FontError> {
        ttf_parser::Font::from_data(bytes, 0).map(|font| Font { font, glyphs: HashMap::new() })
    }

    pub fn build_glyph(&self, glyph: GlyphKey) -> Path {
        use ttf_parser::OutlineBuilder;

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
            fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {}
            fn close(&mut self) {}
        }

        let mut builder = Builder { path: PathBuilder::new() };
        self.font.outline_glyph(GlyphId(glyph.0), &mut builder);
        builder.path.build()
    }

    pub fn measure(&self, text: &str, size: f32) -> (f32, f32) {
        let scale = size / self.font.units_per_em().unwrap() as f32;
        let mut width: f32 = 0.0;
        let mut line_width: f32 = 0.0;
        let mut lines: usize = 0;
        for c in text.chars() {
            if c == '\n' {
                width = width.max(line_width);
                line_width = 0.0;
                lines += 1;
            } else if let Ok(glyph_id) = self.font.glyph_index(c) {
                line_width += scale * self.font.glyph_hor_metrics(glyph_id).unwrap().advance as f32;
            }
        }

        (width, scale * (lines as f32 * self.font.height() as f32 + lines.saturating_sub(1) as f32 * self.font.line_gap() as f32))
    }

    pub fn layout<'f, 't>(&'f self, text: &'t str, size: f32) -> LayoutIter<'f, 't> {
        let scale = size / self.font.units_per_em().unwrap() as f32;
        let origin = Vec2::new(0.0, scale * self.font.ascender() as f32);
        LayoutIter {
            font: self,
            chars: text.chars(),
            scale,
            origin,
            position: origin,
        }
    }
}

pub struct LayoutIter<'f, 'c> {
    font: &'f Font<'f>,
    chars: std::str::Chars<'c>,
    scale: f32,
    origin: Vec2,
    position: Vec2,
}

impl<'f, 'c> Iterator for LayoutIter<'f, 'c> {
    type Item = Glyph;

    fn next(&mut self) -> Option<Glyph> {
        while let Some(c) = self.chars.next() {
            if c == '\n' {
                self.position.x = self.origin.x;
                self.position.y += self.scale * (self.font.font.height() + self.font.font.line_gap()) as f32;
            } else if let Ok(glyph_id) = self.font.font.glyph_index(c) {
                let glyph = Glyph { position: self.position, scale: self.scale, glyph_key: GlyphKey(glyph_id.0) };
                self.position.x += self.scale * self.font.font.glyph_hor_metrics(glyph_id).unwrap().advance as f32;
                return Some(glyph);
            }
        }

        None
    }
}

pub struct Glyph {
    pub position: Vec2,
    pub scale: f32,
    pub glyph_key: GlyphKey,
}
