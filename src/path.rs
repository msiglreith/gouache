use crate::geom::*;
use crate::frame::PathKey;

use std::cell::Cell;

pub struct Path {
    pub(crate) key: Cell<PathKey>,
    pub(crate) offset: Vec2,
    pub(crate) size: Vec2,
    pub(crate) vertices: Vec<Vec2>,
    pub(crate) indices: Vec<usize>,
}

pub struct PathBuilder {
    first: Vec2,
    last: Vec2,
    vertices: Vec<Vec2>,
    indices: Vec<usize>,
}

impl PathBuilder {
    pub fn new() -> PathBuilder {
        PathBuilder {
            first: Vec2::new(0.0, 0.0),
            last: Vec2::new(0.0, 0.0),
            vertices: Vec::new(),
            indices: Vec::new(),
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

    pub fn build(&mut self) -> Path {
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

        Path {
            key: Cell::new(PathKey::NONE),
            vertices: std::mem::replace(&mut self.vertices, Vec::new()),
            indices: std::mem::replace(&mut self.indices, Vec::new()),
            offset,
            size,
        }
    }
}
