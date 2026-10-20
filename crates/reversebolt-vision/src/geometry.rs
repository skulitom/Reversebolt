//! Image coordinates and interpolation. No application-specific geometry.
use image::{Rgb, RgbImage};

pub type Quad = [[f32; 2]; 4];

pub fn bounds(q: Quad) -> [f32; 4] {
    let left = q.iter().map(|p| p[0]).fold(f32::INFINITY, f32::min);
    let top = q.iter().map(|p| p[1]).fold(f32::INFINITY, f32::min);
    let right = q.iter().map(|p| p[0]).fold(f32::NEG_INFINITY, f32::max);
    let bottom = q.iter().map(|p| p[1]).fold(f32::NEG_INFINITY, f32::max);
    [left, top, right - left, bottom - top]
}

pub fn rectangle([x, y, w, h]: [f32; 4]) -> Quad {
    [[x, y], [x + w, y], [x + w, y + h], [x, y + h]]
}

pub fn overlap(a: [f32; 4], b: [f32; 4]) -> bool {
    let intersection = (a[0] + a[2]).min(b[0] + b[2]) - a[0].max(b[0]);
    let height = (a[1] + a[3]).min(b[1] + b[3]) - a[1].max(b[1]);
    intersection.max(0.) * height.max(0.) > 0.5 * (a[2] * a[3]).min(b[2] * b[3])
}

pub fn distance(a: [f32; 2], b: [f32; 2]) -> f32 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

pub fn ordered(mut q: Quad) -> Quad {
    q.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    if q[0][1] > q[1][1] {
        q.swap(0, 1);
    }
    if q[2][1] > q[3][1] {
        q.swap(2, 3);
    }
    [q[0], q[2], q[3], q[1]]
}

pub struct Projection([f32; 8]);

impl Projection {
    /// Map the unit square to a convex quadrilateral.
    pub fn new(q: Quad) -> Self {
        let dx1 = q[1][0] - q[2][0];
        let dx2 = q[3][0] - q[2][0];
        let dx3 = q[0][0] - q[1][0] + q[2][0] - q[3][0];
        let dy1 = q[1][1] - q[2][1];
        let dy2 = q[3][1] - q[2][1];
        let dy3 = q[0][1] - q[1][1] + q[2][1] - q[3][1];
        let determinant = dx1 * dy2 - dx2 * dy1;
        let (g, h) = if determinant.abs() < 1e-6 {
            (0., 0.)
        } else {
            (
                (dx3 * dy2 - dx2 * dy3) / determinant,
                (dx1 * dy3 - dx3 * dy1) / determinant,
            )
        };
        Self([
            q[1][0] - q[0][0] + g * q[1][0],
            q[3][0] - q[0][0] + h * q[3][0],
            q[0][0],
            q[1][1] - q[0][1] + g * q[1][1],
            q[3][1] - q[0][1] + h * q[3][1],
            q[0][1],
            g,
            h,
        ])
    }

    pub fn point(&self, u: f32, v: f32) -> [f32; 2] {
        let [a, b, c, d, e, f, g, h] = self.0;
        let divisor = g * u + h * v + 1.;
        [(a * u + b * v + c) / divisor, (d * u + e * v + f) / divisor]
    }

    pub fn strip(&self, left: f32, right: f32) -> Quad {
        [
            self.point(left, 0.),
            self.point(right, 0.),
            self.point(right, 1.),
            self.point(left, 1.),
        ]
    }
}

fn pixel(image: &RgbImage, x: i32, y: i32, c: usize) -> f32 {
    image.get_pixel(
        x.clamp(0, image.width() as i32 - 1) as u32,
        y.clamp(0, image.height() as i32 - 1) as u32,
    )[c] as f32
}

pub fn resize(image: &RgbImage, width: u32, height: u32) -> RgbImage {
    RgbImage::from_fn(width, height, |x, y| {
        let sx = ((x as f32 + 0.5) * image.width() as f32 / width as f32 - 0.5).max(0.);
        let sy = ((y as f32 + 0.5) * image.height() as f32 / height as f32 - 0.5).max(0.);
        let (ix, iy) = (sx.floor() as i32, sy.floor() as i32);
        let (dx, dy) = (sx - ix as f32, sy - iy as f32);
        Rgb(std::array::from_fn(|c| {
            let top = pixel(image, ix, iy, c) * (1. - dx) + pixel(image, ix + 1, iy, c) * dx;
            let bottom =
                pixel(image, ix, iy + 1, c) * (1. - dx) + pixel(image, ix + 1, iy + 1, c) * dx;
            (top * (1. - dy) + bottom * dy).round().clamp(0., 255.) as u8
        }))
    })
}

fn cubic(x: f32) -> f32 {
    let x = x.abs();
    if x <= 1. {
        (1.25 * x - 2.25) * x * x + 1.
    } else if x < 2. {
        ((-0.75 * x + 3.75) * x - 6.) * x + 3.
    } else {
        0.
    }
}

pub fn crop(image: &RgbImage, q: Quad) -> Option<(RgbImage, Quad)> {
    let w = distance(q[0], q[1]).max(distance(q[2], q[3])) as u32;
    let h = distance(q[0], q[3]).max(distance(q[1], q[2])) as u32;
    if w < 2 || h < 2 || u64::from(w) * u64::from(h) > 2_100_000 {
        return None;
    }
    let projection = Projection::new(q);
    let out = RgbImage::from_fn(w, h, |x, y| {
        let p = projection.point(x as f32 / w as f32, y as f32 / h as f32);
        let sx = (p[0] * 32.).round() / 32.;
        let sy = (p[1] * 32.).round() / 32.;
        let (ix, iy) = (sx.floor() as i32, sy.floor() as i32);
        Rgb(std::array::from_fn(|c| {
            let mut value = 0.;
            for dy in -1..=2 {
                for dx in -1..=2 {
                    value += pixel(image, ix + dx, iy + dy, c)
                        * cubic(sx - (ix + dx) as f32)
                        * cubic(sy - (iy + dy) as f32);
                }
            }
            value.round().clamp(0., 255.) as u8
        }))
    });
    if h as f32 / w as f32 >= 1.5 {
        Some((image::imageops::rotate270(&out), [q[1], q[2], q[3], q[0]]))
    } else {
        Some((out, q))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn perspective_and_word_strips_preserve_geometry() {
        let q = [[10., 15.], [95., 5.], [84., 48.], [15., 52.]];
        let p = Projection::new(q);
        for (actual, expected) in p.strip(0., 1.).into_iter().zip(q) {
            assert!(distance(actual, expected) < 0.001);
        }
        let left = bounds(p.strip(0., 0.4));
        let right = bounds(p.strip(0.6, 1.));
        assert!(!overlap(left, right));
    }
    #[test]
    fn resizing_constant_images_and_edge_crops_preserves_color() {
        let image = RgbImage::from_pixel(4, 3, Rgb([13, 125, 241]));
        for size in [(1, 1), (8, 9), (4, 3)] {
            assert!(
                resize(&image, size.0, size.1)
                    .pixels()
                    .all(|p| p.0 == [13, 125, 241])
            );
        }
        let (out, _) = crop(&image, rectangle([-1., -1., 7., 6.])).unwrap();
        assert!(out.pixels().all(|p| p.0 == [13, 125, 241]));
    }
}
