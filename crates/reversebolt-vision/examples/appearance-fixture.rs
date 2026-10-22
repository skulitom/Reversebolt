//! Declared image controls; the scorer's truth is never supplied to the matcher.
use image::{GrayImage, Luma, imageops};
use reversebolt_core::visual::{self, FrameRef, ObservationInput, Quality, VisualValue};
use serde_json::json;
use std::{fs, path::PathBuf};

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
    let mut image = GrayImage::from_pixel(512, 288, Luma([20 + bright]));
    let patch = GrayImage::from_fn(64, 64, |x, y| Luma([patch.get_pixel(x, y)[0] + bright]));
    imageops::replace(&mut image, &patch, i64::from(at[0]), i64::from(at[1]));
    image
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 1 {
        return Err("usage: appearance-fixture NEW_DIRECTORY".into());
    }
    let output = PathBuf::from(&args[0]);
    fs::create_dir(&output)?;
    let patch = texture(17);
    let before = frame(&patch, [64, 80], 0);
    let mut duplicate = frame(&patch, [224, 144], 0);
    imageops::replace(&mut duplicate, &patch, 352, 48);
    let flat = GrayImage::from_pixel(512, 288, Luma([60]));
    let cases = [
        (
            "Far translation with brightness",
            before.clone(),
            frame(&patch, [224, 144], 20),
            Some([160., 64.]),
        ),
        (
            "Stationary appearance",
            before.clone(),
            before.clone(),
            Some([0., 0.]),
        ),
        ("Repeated appearance", before.clone(), duplicate, None),
        ("Flat appearance", flat.clone(), flat, None),
        (
            "Replaced appearance",
            before,
            frame(&texture(811), [224, 144], 0),
            None,
        ),
    ];
    let mut manifest = vec![];
    let mut truth = vec![];
    for (index, (name, a, b, expected)) in cases.into_iter().enumerate() {
        let investigation = format!("fixture-{index}");
        let mut frames = vec![];
        let mut references = vec![];
        for (number, image) in [(1, a), (2, b)] {
            let path = output.join(format!("case-{index}-{number}.png"));
            image.save(&path)?;
            let reference: FrameRef = serde_json::from_value(
                json!({"id":format!("image-{index}-{number}"),"number":number,
                "sha256":visual::digest(&fs::read(&path)?),"width":512,"height":288,"epoch":0,"started_unix_ms":number}),
            )?;
            frames.push(json!({"file":path,"reference":reference}));
            references.push(reference);
        }
        let mut tracks = vec![];
        visual::record(
            &mut tracks,
            ObservationInput {
                investigation_id: investigation.clone(),
                frame_id: references[0].id.clone(),
                image_sha256: references[0].sha256.clone(),
                observer: "analytic fixture".into(),
                key: "region".into(),
                label: "Observed region".into(),
                unit: String::new(),
                value: Some(VisualValue::Text("appearance".into())),
                quality: Quality::Provisional,
                bounds: Some([64., 80., 64., 64.]),
                explanation: "An image region from a declared analytic fixture.".into(),
            },
            &references[0],
            "fixture",
            2,
        )?;
        manifest.push(json!({"name":format!("case-{index}"),"investigation_id":investigation,"frames":frames,"visual_tracks":tracks}));
        truth.push(json!({"name":format!("case-{index}"),"description":name,"expected_displacement":expected}));
    }
    for (name, value) in [
        ("cases.json", json!(manifest)),
        ("truth.json", json!(truth)),
    ] {
        let file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output.join(name))?;
        serde_json::to_writer_pretty(file, &value)?;
    }
    println!("{}", json!({"cases":manifest.len(),"target_access":false}));
    Ok(())
}
