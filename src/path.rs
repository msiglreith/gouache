use crate::geom::*;
use crate::frame::*;

pub struct Path {
    pub(crate) offset: Vec2,
    pub(crate) size: Vec2,
    pub(crate) buffer: Vec<[u16; 3]>,
}


#[derive(Copy, Clone)]
pub struct Segment {
    p1: Vec2,
    p2: Vec2,
    p3: Vec2,
}

impl Segment {
    fn split_at(&self, t: f32) -> (Segment, Segment) {
        let p12 = Vec2::lerp(t, self.p1, self.p2);
        let p23 = Vec2::lerp(t, self.p2, self.p3);
        let point = Vec2::lerp(t, p12, p23);
        (
            Segment { p1: self.p1, p2: p12, p3: point },
            Segment { p1: point, p2: p23, p3: self.p3 },
        )
    }

    fn split_to_monotone(&self) -> [Option<Segment>; 3] {
        fn monotone(x1: f32, x2: f32, x3: f32) -> bool {
            (x1 <= x2 && x2 <= x3) || (x3 <= x2 && x2 <= x1)
        }

        fn stationary_point(x1: f32, x2: f32, x3: f32) -> f32 {
            (x1 - x2) / (x1 - 2.0 * x2 + x3)
        }

        let x_split = if monotone(self.p1.x, self.p2.x, self.p3.x) {
            None
        } else {
            Some(stationary_point(self.p1.x, self.p2.x, self.p3.x))
        };

        let y_split = if monotone(self.p1.y, self.p2.y, self.p3.y) {
            None
        } else {
            Some(stationary_point(self.p1.y, self.p2.y, self.p3.y))
        };

        match (x_split, y_split) {
            (None, None) => {
                [Some(*self), None, None]
            }
            (Some(split), None) | (None, Some(split)) => {
                let (s1, s2) = self.split_at(split);
                [Some(s1), Some(s2), None]
            }
            (Some(x_split), Some(y_split)) => {
                let (split1, split2) = (x_split.min(y_split), x_split.max(y_split));
                let (s1, s2) = self.split_at(split1);
                let (s2, s3) = s2.split_at((split2 - split1) / (1.0 - split1));
                [Some(s1), Some(s2), Some(s3)]
            }
        }
    }
}

impl Path {
    pub fn build(segments: &[Segment]) -> Path {
        let mut segments_monotone = Vec::with_capacity(segments.len());
        let mut last = Vec2::new(0.0, 0.0);
        for segment in segments {
            let [s1, s2, s3] = segment.split_to_monotone();
            if let Some(s1) = s1 { segments_monotone.push(s1); }
            if let Some(s2) = s2 { segments_monotone.push(s2); }
            if let Some(s3) = s3 { segments_monotone.push(s3); }
        }

        let mut min = Vec2::new(std::f32::INFINITY, std::f32::INFINITY);
        let mut max = Vec2::new(-std::f32::INFINITY, -std::f32::INFINITY);
        for segment in segments_monotone.iter() {
            min = min.min(segment.p1).min(segment.p2).min(segment.p3);
            max = max.max(segment.p1).max(segment.p2).max(segment.p3);
        }
        if !min.x.is_finite() { min.x = 0.0; }
        if !max.x.is_finite() { max.x = 0.0; }
        if !min.y.is_finite() { min.y = 0.0; }
        if !max.y.is_finite() { max.y = 0.0; }

        let offset = min;
        let size = max - min;

        fn convert(vertex: Vec2, offset: Vec2, size: Vec2) -> (u16, u16) {
            let scaled = (std::u16::MAX - 1) as f32 * Vec2::new((vertex.x - offset.x) / size.x, (vertex.y - offset.y) / size.y);
            (scaled.x.round() as u16 + 1, scaled.y.round() as u16 + 1)
        }

        let mut buffer = Vec::with_capacity(segments_monotone.len());
        for segment in segments_monotone.iter() {
            let (x1, y1) = convert(segment.p1, offset, size);
            let (x2, y2) = convert(segment.p2, offset, size);
            let (x3, y3) = convert(segment.p3, offset, size);
            buffer.push([x1, y1, x2]);
            buffer.push([y2, x3, y3]);
        }

        Path {
            offset,
            size,
            buffer,
        }
    }
}

impl Path {
    pub fn get_quad(&self, position: Vec2, transform: Mat2x2) -> Quad {
        let p = position + transform * Vec2::new(self.offset.x, self.offset.y);
        let v1 = transform * Vec2::new(self.size.x, 0.0);
        let v2 = transform * Vec2::new(0.0, self.size.y);
        let n1 = v1.normalized();
        let n2 = v2.normalized();

        let d = 0.5 / n1.cross(n2).abs();
        let d1 = d * n1;
        let d2 = d * n2;

        let dx = d / v1.length();
        let dy = d / v2.length();

        Quad {
            vertices: [
                p - d1 - d2,
                p + v1 + d1 - d2,
                p + v1 + d1 + v2 + d2,
                p - d1 + v2 + d2,
            ],
            uv: [
                Vec2::new(-dx, -dy),
                Vec2::new(1.0 + dx, -dy),
                Vec2::new(1.0 + dx, 1.0 + dy),
                Vec2::new(-dx, 1.0 + dy),
            ],
        }
    }
}

pub struct Quad {
    pub vertices: [Vec2; 4],
    pub uv: [Vec2; 4],
}

pub struct PathBuilder {
    segments: Vec<Segment>,
    first: Vec2,
    last: Vec2,
}

impl PathBuilder {
    pub fn new() -> PathBuilder {
        PathBuilder {
            segments: Vec::new(),
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
        self.segments.push(Segment {
            p1: self.last,
            p2: 0.5 * (self.last + point),
            p3: point,
        });
        self.last = point;
        self
    }

    pub fn quadratic_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32) -> &mut Self {
        self.segments.push(Segment {
            p1: self.last,
            p2: Vec2::new(x1, y1),
            p3: Vec2::new(x2, y2),
        });
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
    }

    pub fn build(&mut self) -> Path {
        self.close();

        Path::build(&self.segments)
    }
}
