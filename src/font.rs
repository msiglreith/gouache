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

    pub fn layout(&self, text: &str, size: f32) -> TextLayout {
        let mut glyphs = Vec::new();

        let scale = size / self.font.units_per_em().unwrap() as f32;
        let mut position = Vec2::new(0.0, scale * self.font.ascender() as f32);
        for c in text.chars() {
            if let Ok(glyph_id) = self.font.glyph_index(c) {
                glyphs.push(Glyph { position, glyph_key: GlyphKey(glyph_id.0) });
                position.x += scale * self.font.glyph_hor_metrics(glyph_id).unwrap().advance as f32;
            }
        }

        TextLayout { scale, glyphs }
    }
}

pub struct TextLayout {
    pub scale: f32,
    pub glyphs: Vec<Glyph>,
}

pub struct Glyph {
    pub position: Vec2,
    pub glyph_key: GlyphKey,
}
