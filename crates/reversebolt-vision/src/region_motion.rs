//! A fixed image-area measurement, not object tracking or a semantic control.
use crate::{patch_motion, response};
use image::imageops::{self, FilterType};
use reversebolt_core::Result;
use serde::{Deserialize, Serialize};

pub const POLICY: &str = "fixed_region_patch_motion_v1";
pub const LIMITATION: &str = "Both images use the same fixed rectangle in original image pixels. Patches are measured in a 256x144 normalization of that rectangle, not of the whole screen. Displacement multiplied by displacement_to_source_scale approximates displacement in source pixels. This is not an object identity, tracked crop, actor movement, physical speed or gameplay effect. Objects can enter or leave the rectangle. Resizing, occlusion, weak texture and large motion can remain unresolved. No missing match or zero displacement proves no movement.";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Region {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}
impl Region {
    pub fn validate_shape(self) -> Result<()> {
        if self.width < 32
            || self.height < 32
            || u64::from(self.width) * u64::from(self.height) > 4_000_000
            || self.x.checked_add(self.width).is_none()
            || self.y.checked_add(self.height).is_none()
        {
            return Err("Choose an image region at least 32 pixels wide and high, within the four-million-pixel budget.".into());
        }
        Ok(())
    }
    fn validate(self, dimensions: [u32; 2]) -> Result<()> {
        self.validate_shape()?;
        if self.x + self.width > dimensions[0] || self.y + self.height > dimensions[1] {
            return Err("The fixed region lies outside the captured image.".into());
        }
        Ok(())
    }
}

#[derive(Debug, Serialize)]
pub struct Report {
    pub policy: &'static str,
    pub source_dimensions: [u32; 2],
    pub region: Region,
    pub displacement_to_source_scale: [f64; 2],
    pub measurement: patch_motion::Report,
    pub limitation: &'static str,
}

pub fn measure(before: &[u8], after: &[u8], region: Region) -> Result<Report> {
    region.validate_shape()?;
    let a = response::grayscale(before)?;
    let b = response::grayscale(after)?;
    if a.dimensions() != b.dimensions() {
        return Err("Image geometry changed.".into());
    }
    let source_dimensions = [a.width(), a.height()];
    region.validate(source_dimensions)?;
    let crop = |image| {
        imageops::resize(
            &imageops::crop_imm(image, region.x, region.y, region.width, region.height).to_image(),
            256,
            144,
            FilterType::Triangle,
        )
    };
    Ok(Report {
        policy: POLICY,
        source_dimensions,
        region,
        displacement_to_source_scale: [
            f64::from(region.width) / 256.,
            f64::from(region.height) / 144.,
        ],
        measurement: patch_motion::prepared(crop(&a), crop(&b), [region.width, region.height]),
        limitation: LIMITATION,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{GrayImage, Luma};
    fn png(image: &GrayImage) -> Vec<u8> {
        let mut file = std::io::Cursor::new(vec![]);
        image.write_to(&mut file, image::ImageFormat::Png).unwrap();
        file.into_inner()
    }
    fn texture(width: u32, height: u32, seed: u32) -> GrayImage {
        GrayImage::from_fn(width, height, |x, y| {
            let mut n = (x + y * width)
                .wrapping_add(seed)
                .wrapping_mul(747796405)
                .wrapping_add(2891336453);
            n = ((n >> ((n >> 28) + 4)) ^ n).wrapping_mul(277803737);
            Luma([32 + (((n >> 22) ^ n) % 128) as u8])
        })
    }
    #[test]
    fn a_small_moving_area_is_measured_separately_from_the_static_background() {
        // Keep background features visible after full-screen normalization;
        // independent per-pixel noise at 1024x576 can disappear when reduced.
        let background = imageops::resize(&texture(256, 144, 4), 1024, 576, FilterType::Nearest);
        let local = texture(256, 144, 9);
        let region = Region {
            x: 384,
            y: 216,
            width: 256,
            height: 144,
        };
        let scene = |delta: i32| {
            let mut image = background.clone();
            for y in 0..region.height {
                for x in 0..region.width {
                    let sx = x as i32 - delta;
                    image.put_pixel(
                        region.x + x,
                        region.y + y,
                        if (0..256).contains(&sx) {
                            *local.get_pixel(sx as u32, y)
                        } else {
                            Luma([0])
                        },
                    );
                }
            }
            png(&image)
        };
        let before = scene(0);
        for shift in [8, -8] {
            let after = scene(shift);
            let global = patch_motion::measure(&before, &after).unwrap();
            assert_eq!(global.consensus.unwrap().displacement, [0, 0]);
            let local = measure(&before, &after, region).unwrap();
            assert_eq!(
                local.measurement.consensus.unwrap().displacement,
                [shift, 0]
            );
            assert_eq!(local.source_dimensions, [1024, 576]);
            assert_eq!(local.displacement_to_source_scale, [1., 1.]);
        }
        // Full-image scope is exactly the existing measurement, not a new metric.
        let full = measure(
            &before,
            &before,
            Region {
                x: 0,
                y: 0,
                width: 1024,
                height: 576,
            },
        )
        .unwrap();
        assert_eq!(
            serde_json::to_value(full.measurement).unwrap(),
            serde_json::to_value(patch_motion::measure(&before, &before).unwrap()).unwrap()
        );
    }
    #[test]
    fn invalid_or_empty_regions_and_weak_texture_never_invent_movement() {
        let bytes = png(&GrayImage::from_pixel(128, 72, Luma([80])));
        let region = Region {
            x: 0,
            y: 0,
            width: 128,
            height: 72,
        };
        let empty = measure(&bytes, &bytes, region).unwrap();
        assert_eq!(empty.displacement_to_source_scale, [0.5, 0.5]);
        assert_eq!(empty.measurement.selection.selected_patches, 0);
        assert!(empty.measurement.consensus.is_none());
        for bad in [
            Region { width: 0, ..region },
            Region {
                height: 31,
                ..region
            },
            Region { x: 1, ..region },
            Region {
                x: u32::MAX,
                ..region
            },
            Region {
                width: 4000000,
                ..region
            },
        ] {
            assert!(measure(&bytes, &bytes, bad).is_err());
        }
        assert!(measure(&bytes, &png(&GrayImage::new(256, 144)), region).is_err());
        assert!(measure(b"invalid", &bytes, region).is_err());
    }
}
