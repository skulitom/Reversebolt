use super::*;
use image::Luma;

fn texture(seed: u32) -> GrayImage {
    GrayImage::from_fn(64, 64, |x, y| {
        let mut n = (x + y * 64)
            .wrapping_add(seed)
            .wrapping_mul(747796405)
            .wrapping_add(2891336453);
        n = ((n >> ((n >> 28) + 4)) ^ n).wrapping_mul(277803737);
        Luma([40 + (((n >> 22) ^ n) % 140) as u8])
    })
}
fn frame(patch: &GrayImage, at: [u32; 2], bright: u8) -> GrayImage {
    let mut image = GrayImage::from_pixel(WIDTH, HEIGHT, Luma([20 + bright]));
    let patch = GrayImage::from_fn(patch.width(), patch.height(), |x, y| {
        Luma([patch.get_pixel(x, y)[0] + bright])
    });
    imageops::replace(&mut image, &patch, i64::from(at[0]), i64::from(at[1]));
    image
}
fn region() -> Region {
    Region {
        x: 64,
        y: 80,
        width: 64,
        height: 64,
    }
}
fn png(image: &GrayImage) -> Vec<u8> {
    let mut bytes = std::io::Cursor::new(vec![]);
    image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
    bytes.into_inner()
}

#[test]
fn full_frame_search_follows_large_translation_brightness_and_stationary_templates() {
    let source = frame(&texture(1), [64, 80], 0);
    for (at, bright) in [([224, 144], 20), ([32, 32], 0), ([64, 80], 0)] {
        let target = frame(&texture(1), at, bright);
        let report = measure(&png(&source), &png(&target), region()).unwrap();
        assert_eq!(
            serde_json::to_value(&report).unwrap(),
            serde_json::to_value(prepared(&source, &target, region(), [WIDTH, HEIGHT])).unwrap()
        );
        assert_eq!(report.status, Status::Matched, "{report:?}");
        assert_eq!(
            report.displacement,
            Some([f64::from(at[0]) - 64., f64::from(at[1]) - 80.])
        );
        assert_eq!(
            report.matched_region,
            Some([f64::from(at[0]), f64::from(at[1]), 64., 64.])
        );
        assert!(report.forward.as_ref().unwrap().positions <= (WIDTH * HEIGHT) as usize);
        assert!(report.forward.as_ref().unwrap().samples <= 256);
        assert!(report.reverse.as_ref().unwrap().dense_ncc.unwrap() > 0.999);
    }
}

#[test]
fn repeated_appearance_and_reverse_ambiguity_do_not_establish_a_correspondence() {
    let patch = texture(1);
    let single = frame(&patch, [64, 80], 0);
    let mut duplicate = single.clone();
    imageops::replace(&mut duplicate, &patch, 320, 80);
    let forward = prepared(&single, &duplicate, region(), [WIDTH, HEIGHT]);
    assert_eq!(forward.status, Status::Ambiguous);
    assert!(forward.matched_region.is_none() && forward.displacement.is_none());
    let reverse = prepared(&duplicate, &single, region(), [WIDTH, HEIGHT]);
    assert_eq!(reverse.status, Status::InconsistentReverse);
    assert_eq!(reverse.reverse.unwrap().status, Status::Ambiguous);
    assert!(reverse.matched_region.is_none() && reverse.displacement.is_none());
}

#[test]
fn sparse_sample_aliases_flat_missing_and_edge_templates_are_not_promoted() {
    let patch = texture(1);
    let source = frame(&patch, [64, 80], 0);
    let flat = GrayImage::from_pixel(WIDTH, HEIGHT, Luma([20]));
    let low = prepared(&flat, &flat, region(), [WIDTH, HEIGHT]);
    assert_eq!(low.status, Status::LowTexture);
    assert_eq!(low.forward.unwrap().positions, 0);
    for target in [flat, frame(&texture(999), [224, 144], 0)] {
        let report = prepared(&source, &target, region(), [WIDTH, HEIGHT]);
        assert_ne!(report.status, Status::Matched);
        assert!(report.displacement.is_none());
    }
    let at_boundary = prepared(
        &source,
        &frame(&patch, [0, 80], 0),
        region(),
        [WIDTH, HEIGHT],
    );
    assert_eq!(at_boundary.status, Status::ImageBoundary);
    let mut alias = GrayImage::from_fn(64, 64, |x, y| Luma([220 - patch.get_pixel(x, y)[0]]));
    for iy in 0..16 {
        for ix in 0..16 {
            let (x, y) = (ix * 63 / 15, iy * 63 / 15);
            alias.put_pixel(x, y, *patch.get_pixel(x, y));
        }
    }
    let report = prepared(
        &source,
        &frame(&alias, [224, 144], 0),
        region(),
        [WIDTH, HEIGHT],
    );
    assert_eq!(report.status, Status::PoorMatch);
    assert!(report.forward.as_ref().unwrap().best_ncc.unwrap() > 0.999);
    assert!(report.forward.unwrap().dense_ncc.unwrap() < MIN_NCC);
    assert!(report.displacement.is_none());
}

#[test]
fn geometry_and_normalized_size_limits_are_explicit() {
    let image = GrayImage::from_pixel(3072, 1024, Luma([80]));
    let bytes = png(&image);
    let too_small = measure(
        &bytes,
        &bytes,
        Region {
            x: 0,
            y: 0,
            width: 32,
            height: 32,
        },
    )
    .unwrap();
    // 32 source pixels become fewer than eight pixels horizontally.
    assert_eq!(too_small.status, Status::UnsupportedSize);
    let huge = measure(
        &bytes,
        &bytes,
        Region {
            x: 0,
            y: 0,
            width: 3072,
            height: 1024,
        },
    )
    .unwrap();
    assert_eq!(huge.status, Status::UnsupportedSize);
    assert!(measure(&bytes, &png(&GrayImage::new(512, 288)), region()).is_err());
    assert!(
        measure(
            &bytes,
            &bytes,
            Region {
                x: 3060,
                y: 0,
                width: 32,
                height: 32
            }
        )
        .is_err()
    );
    assert!(measure(b"invalid", &bytes, region()).is_err());
}
