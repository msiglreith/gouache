use crate::render::*;

const TOLERANCE: f32 = 0.1;

pub struct Graphics {
    renderer: Renderer,
    width: f32,
    height: f32,
    color: Color,
}

impl Graphics {
    pub fn new(width: f32, height: f32) -> Graphics {
        let mut renderer = Renderer::new(width as u32, height as u32);
        Graphics {
            renderer,
            width,
            height,
            color: Color::rgba(1.0, 1.0, 1.0, 1.0),
        }
    }

    pub fn set_size(&mut self, width: f32, height: f32) {
        self.width = width;
        self.height = height;

        self.renderer.resize(width as u32, height as u32);
    }

    pub fn clear(&mut self, color: Color) {
        self.renderer.clear(color.to_linear_premul(), &RenderOptions::default());
    }

    pub fn begin_frame(&mut self) {}

    pub fn end_frame(&mut self) {}

    pub fn draw_path(&mut self) {
        self.renderer.draw(&[
            Vertex { pos: [0.0, 0.0], uv: [0.0, 0.0], path: [0, 11] },
            Vertex { pos: [0.5 * 0.0655, 0.0], uv: [1.0, 0.0], path: [0, 11] },
            Vertex { pos: [0.5 * 0.0655, 0.5 * 0.0728], uv: [1.0, 1.0], path: [0, 11] },
            Vertex { pos: [0.0, 0.5 * 0.0728], uv: [0.0, 1.0], path: [0, 11] },
            // Vertex { pos: [0.0 - 0.25, 0.728], uv: [0.0, 0.0], path: [0, 11] },
            // Vertex { pos: [0.0 - 0.25, 0.0], uv: [1.0, 0.0], path: [0, 11] },
            // Vertex { pos: [0.655 - 0.25, 0.0 + 0.4], uv: [1.0, 1.0], path: [0, 11] },
            // Vertex { pos: [0.655 - 0.25, 0.728 + 0.4], uv: [0.0, 1.0], path: [0, 11] },
            Vertex { pos: [0.0 - 1.0, 0.0 - 1.0], uv: [0.0, 0.0], path: [12,60] },
            Vertex { pos: [0.655 - 1.0, 0.0 - 1.0], uv: [1.0, 0.0], path: [12,60] },
            Vertex { pos: [0.655 - 1.0, 0.728 - 1.0], uv: [1.0, 1.0], path: [12,60] },
            Vertex { pos: [0.0 - 1.0, 0.728 - 1.0], uv: [0.0, 1.0], path: [12,60] },
        ], &[0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7, /*8, 9, 10, 8, 10, 11*/], &RenderOptions::default());
        // self.renderer.draw(&[
        //     Vertex { pos: [0.0, 0.0], uv: [0.0, 0.0], path: [0, 1] },
        //     Vertex { pos: [0.5, 0.0], uv: [1.0, 0.0], path: [0, 1] },
        //     Vertex { pos: [0.5, 0.5], uv: [1.0, 1.0], path: [0, 1] },
        //     Vertex { pos: [0.0, 0.5], uv: [0.0, 1.0], path: [0, 1] },
        // ], &[0, 1, 2, 0, 2, 3], &RenderOptions::default());
    }

    pub fn set_color(&mut self, color: Color) {
        self.color = color;
    }
}

pub struct Path {}

impl Path {
    pub fn new() -> Path {
        Path {}
    }

    pub fn move_to(&mut self, point: Point) -> &mut Self {
        self
    }

    pub fn line_to(&mut self, point: Point) -> &mut Self {
        self
    }

    pub fn quadratic_to(&mut self, control: Point, point: Point) -> &mut Self {
        self
    }

    pub fn cubic_to(&mut self, control1: Point, control2: Point, point: Point) -> &mut Self {
        self
    }

    pub fn arc_to(&mut self, radius: f32, point: Point) -> &mut Self {
        self
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
    if x < 0.04045 { x / 12.92 } else { ((x + 0.055)/1.055).powf(2.4)  }
}

use std::ops;

#[derive(Copy, Clone, Debug)]
pub struct Point { pub x: f32, pub y: f32 }

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
    pub fn dot(self, other: Point) -> f32 {
        self.x * other.x + self.y * other.y
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
