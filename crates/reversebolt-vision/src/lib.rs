//! Bounded, image-only PP-OCRv6 inference using the pure-Rust RTen runtime.
//! Model metadata and dictionary attribution are in NOTICE.md. Observations are
//! provisional: neither a model score nor a stable region verifies meaning.
mod geometry;

use geometry::{Projection, Quad, bounds, crop, distance, ordered, overlap, rectangle, resize};
use image::RgbImage;
use reversebolt_core::{Result, live::parse_number};
use rten_imageproc::{BoundingRect, PointF, RetrievalMode, find_contours, min_area_rect};
use rten_tensor::{NdTensor, prelude::*};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{path::Path, sync::Arc};

const DETECTOR: &str = "PP-OCRv6_medium_det_onnx";
const RECOGNIZER: &str = "PP-OCRv6_medium_rec_onnx";
const MAX_PIXELS: u64 = 2_100_000;
const MAX_DETECTION_PIXELS: u64 = 4_000_000;
const MAX_LINES: usize = 256;
const MAX_RECOGNITION_WIDTH: usize = 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    Detected,
    Revisited,
}

#[derive(Clone, Debug, Serialize)]
pub struct Word {
    pub text: String,
    pub polygon: Quad,
    /// Uncalibrated mean of selected CTC emission probabilities.
    pub score: f32,
    pub source: Source,
    pub line: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct TextLine {
    pub text: String,
    pub polygon: Quad,
    pub score: f32,
    pub words: Vec<Word>,
}

#[derive(Serialize)]
pub struct Reading {
    pub words: Vec<Word>,
    pub lines: Vec<TextLine>,
    /// Unsupported regions are missing observations, never empty text or a zero.
    pub skipped_regions: Vec<SkippedRegion>,
    pub elapsed_ms: u128,
}

#[derive(Debug, Serialize)]
pub struct SkippedRegion {
    pub polygon: Quad,
    pub source: Source,
    pub reason: SkipReason,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SkipReason {
    InvalidCrop,
    RecognitionWidth { required: usize, maximum: usize },
}

struct RecognitionCrop {
    image: RgbImage,
    polygon: Quad,
    width: usize,
}

/// Partition before batching so one unsupported line cannot reject its neighbors.
fn prepare_crops(
    image: &RgbImage,
    quads: &[Quad],
    source: Source,
) -> (Vec<RecognitionCrop>, Vec<SkippedRegion>) {
    let mut crops = vec![];
    let mut skipped = vec![];
    for &polygon in quads {
        let Some((image, polygon)) = crop(image, polygon) else {
            skipped.push(SkippedRegion {
                polygon,
                source,
                reason: SkipReason::InvalidCrop,
            });
            continue;
        };
        let width = (u64::from(image.width()) * 48).div_ceil(u64::from(image.height())) as usize;
        if width > MAX_RECOGNITION_WIDTH {
            skipped.push(SkippedRegion {
                polygon,
                source,
                reason: SkipReason::RecognitionWidth {
                    required: width,
                    maximum: MAX_RECOGNITION_WIDTH,
                },
            });
        } else {
            crops.push(RecognitionCrop {
                image,
                polygon,
                width,
            });
        }
    }
    crops.sort_by_key(|crop| crop.width);
    (crops, skipped)
}

struct Region {
    bounds: [f32; 4],
    misses: usize,
}

pub struct Reader {
    detector: rten::Model,
    recognizer: rten::Model,
    dictionary: Vec<&'static str>,
    pool: Arc<rten::ThreadPool>,
    regions: Vec<Region>,
    size: Option<(u32, u32)>,
}

fn load_model(root: &Path, name: &str, bytes: u64, sha: &str) -> Result<rten::Model> {
    let path = root.join(name).join("inference.onnx");
    let metadata = std::fs::metadata(&path).map_err(|e| format!("{name}: {e}"))?;
    if metadata.len() != bytes {
        return Err(format!("Unexpected size for {name}"));
    }
    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    if format!("{:x}", Sha256::digest(&data)) != sha {
        return Err(format!("Checksum mismatch for {name}"));
    }
    rten::Model::load(data).map_err(|e| e.to_string())
}

impl Reader {
    pub fn load(root: &Path) -> Result<Self> {
        let detector = load_model(
            root,
            DETECTOR,
            62032837,
            "eb13b44b25bb36f89528b68720af8a61d9cf381176107f465db1757b65d086e1",
        )?;
        let recognizer = load_model(
            root,
            RECOGNIZER,
            76554979,
            "9c09abf0957f7968c7586464b7397b84ad2387a0497a351af40e9acc71b673ba",
        )?;
        let mut dictionary = vec![""];
        dictionary.extend(include_str!("../assets/characters.txt").lines());
        dictionary.push(" ");
        if dictionary.len() != 18710 {
            return Err("Unexpected PP-OCRv6 dictionary size".into());
        }
        Ok(Self {
            detector,
            recognizer,
            dictionary,
            pool: Arc::new(rten::ThreadPool::with_num_threads(4)),
            regions: vec![],
            size: None,
        })
    }

    fn run(
        &self,
        model: &rten::Model,
        input: &NdTensor<f32, 4>,
    ) -> Result<rten_tensor::Tensor<f32>> {
        let output = model
            .run_one(
                input.view().into(),
                Some(rten::RunOptions::default().with_thread_pool(Some(self.pool.clone()))),
            )
            .map_err(|e| e.to_string())?;
        let output = rten_tensor::Tensor::<f32>::try_from(output).map_err(|e| e.to_string())?;
        Ok(output)
    }

    fn detect(&self, image: &RgbImage) -> Result<Vec<Quad>> {
        let (width, height) = detection_size(image.dimensions())?;
        let scaled = resize(image, width as u32, height as u32);
        let mean = [0.485, 0.456, 0.406];
        let std = [0.229, 0.224, 0.225];
        let input = NdTensor::from_fn([1, 3, height, width], |[_, c, y, x]| {
            (scaled.get_pixel(x as u32, y as u32)[2 - c] as f32 / 255. - mean[c]) / std[c]
        });
        let result = self.run(&self.detector, &input)?;
        if result.shape() != [1, 1, height, width] {
            return Err("Unexpected detector output shape".into());
        }
        let probabilities: Vec<f32> = result.iter().copied().collect();
        if !probabilities.iter().all(|p| p.is_finite()) {
            return Err("Nonfinite detector output".into());
        }
        let mask = NdTensor::from_data(
            [height, width],
            probabilities.iter().map(|p| *p > 0.2).collect::<Vec<_>>(),
        );
        let contours = find_contours(mask.view(), RetrievalMode::List);
        if contours.len() > 3000 {
            return Err("Text detection exceeded 3000 candidate regions".into());
        }
        let mut quads = vec![];
        for contour in contours.iter() {
            let points: Vec<_> = contour.iter().map(|p| p.to_f32()).collect();
            let Some(mut rect) = min_area_rect(&points) else {
                continue;
            };
            if rect.width().min(rect.height()) < 3. {
                continue;
            }
            let bounds = rect.bounding_rect();
            let mut sum = 0.;
            let mut count = 0;
            for y in
                (bounds.top().floor() as usize)..=(bounds.bottom().ceil() as usize).min(height - 1)
            {
                for x in (bounds.left().floor() as usize)
                    ..=(bounds.right().ceil() as usize).min(width - 1)
                {
                    if rect.contains(PointF::from_yx(y as f32, x as f32)) {
                        sum += probabilities[y * width + x];
                        count += 1;
                    }
                }
            }
            if count == 0 || sum / (count as f32) < 0.45 {
                continue;
            }
            // The bounding rectangle of a round-offset rectangle expands by
            // this distance on each side; no dependency on a game or font.
            let expansion = rect.area() * 1.4 / (2. * (rect.width() + rect.height()));
            rect.resize(
                rect.width() + 2. * expansion,
                rect.height() + 2. * expansion,
            );
            if rect.width().min(rect.height()) < 5. {
                continue;
            }
            let q = ordered(rect.corners().map(|p| {
                [
                    (p.x / width as f32 * image.width() as f32)
                        .round()
                        .clamp(0., image.width() as f32 - 1.),
                    (p.y / height as f32 * image.height() as f32)
                        .round()
                        .clamp(0., image.height() as f32 - 1.),
                ]
            }));
            if distance(q[0], q[1]) <= 3. || distance(q[0], q[3]) <= 3. {
                continue;
            }
            quads.push(q);
        }
        if quads.len() > MAX_LINES {
            return Err(format!("Text detection exceeded {MAX_LINES} lines"));
        }
        quads.sort_by(|a, b| {
            a[0][1]
                .total_cmp(&b[0][1])
                .then(a[0][0].total_cmp(&b[0][0]))
        });
        Ok(quads)
    }

    fn recognize(
        &self,
        image: &RgbImage,
        quads: &[Quad],
        source: Source,
    ) -> Result<(Vec<TextLine>, Vec<SkippedRegion>)> {
        let (crops, skipped) = prepare_crops(image, quads, source);
        let mut lines = vec![];
        for batch in crops.chunks(6) {
            let padded_width = batch
                .iter()
                .map(|crop| crop.width)
                .max()
                .unwrap_or(320)
                .max(320);
            let scaled: Vec<_> = batch
                .iter()
                .map(|crop| resize(&crop.image, crop.width as u32, 48))
                .collect();
            let input = NdTensor::from_fn([batch.len(), 3, 48, padded_width], |[n, c, y, x]| {
                if x >= batch[n].width {
                    0.
                } else {
                    scaled[n].get_pixel(x as u32, y as u32)[2 - c] as f32 / 127.5 - 1.
                }
            });
            let result = self.run(&self.recognizer, &input)?;
            let shape = result.shape();
            if shape.len() != 3
                || shape[0] != batch.len()
                || shape[2] != self.dictionary.len()
                || shape[1] == 0
            {
                return Err("Unexpected recognizer output shape".into());
            }
            let steps = shape[1];
            let values: Vec<f32> = result.iter().copied().collect();
            if !values.iter().all(|p| p.is_finite()) {
                return Err("Nonfinite recognition output".into());
            }
            let classes = self.dictionary.len();
            for (n, crop) in batch.iter().enumerate() {
                let tokens = decode(
                    &values[n * steps * classes..(n + 1) * steps * classes],
                    classes,
                    &self.dictionary,
                );
                let score = if tokens.is_empty() {
                    0.
                } else {
                    tokens.iter().map(|t| t.score).sum::<f32>() / tokens.len() as f32
                };
                let text = tokens.iter().map(|t| t.text).collect::<String>();
                let words = if score >= 0.5 {
                    word_spans(&tokens, crop.polygon, steps, padded_width, crop.width)
                } else {
                    vec![]
                };
                lines.push(TextLine {
                    text,
                    polygon: crop.polygon,
                    score,
                    words,
                });
            }
        }
        lines.sort_by(|a, b| {
            a.polygon[0][1]
                .total_cmp(&b.polygon[0][1])
                .then(a.polygon[0][0].total_cmp(&b.polygon[0][0]))
        });
        for (i, line) in lines.iter_mut().enumerate() {
            for w in &mut line.words {
                w.line = i;
            }
        }
        Ok((lines, skipped))
    }

    pub fn read(&mut self, image: &RgbImage) -> Result<Reading> {
        let result = self.read_inner(image);
        if result.is_err() {
            // An unreadable interval cannot keep spatial hints alive indefinitely.
            self.regions.clear();
        }
        result
    }

    fn read_inner(&mut self, image: &RgbImage) -> Result<Reading> {
        let start = std::time::Instant::now();
        if image.width() == 0
            || image.height() == 0
            || u64::from(image.width()) * u64::from(image.height()) > MAX_PIXELS
        {
            return Err("Text recognition requires an image of 1..2100000 pixels".into());
        }
        if self.size != Some(image.dimensions()) {
            self.regions.clear();
        }
        self.size = Some(image.dimensions());
        let quads = self.detect(image)?;
        let (lines, mut skipped_regions) = self.recognize(image, &quads, Source::Detected)?;
        let mut words = lines
            .iter()
            .flat_map(|l| l.words.iter().cloned())
            .collect::<Vec<_>>();
        let numeric = words
            .iter()
            .filter(|w| parse_number(&w.text).is_some())
            .cloned()
            .collect::<Vec<_>>();
        let hints = self
            .regions
            .iter()
            .filter(|r| !numeric.iter().any(|w| overlap(r.bounds, bounds(w.polygon))))
            .map(|r| rectangle(r.bounds))
            .collect::<Vec<_>>();
        let (recovered, skipped_hints) = self.recognize(image, &hints, Source::Revisited)?;
        skipped_regions.extend(skipped_hints);
        for (i, line) in recovered.into_iter().enumerate() {
            if line.score >= 0.5 && parse_number(&line.text).is_some() {
                words.push(Word {
                    text: line.text,
                    polygon: line.polygon,
                    score: line.score,
                    source: Source::Revisited,
                    line: lines.len() + i,
                });
            }
        }
        if words.len() > 512 {
            return Err("Text recognition exceeded 512 word observations".into());
        }
        update_regions(&mut self.regions, &numeric);
        Ok(Reading {
            words,
            lines,
            skipped_regions,
            elapsed_ms: start.elapsed().as_millis(),
        })
    }
}

fn detection_size((w, h): (u32, u32)) -> Result<(usize, usize)> {
    if w == 0 || h == 0 {
        return Err("Empty image".into());
    }
    let ratio = (736. / w.min(h) as f32).max(1.);
    let width = (((w as f32 * ratio) as usize as f32 / 32.).round_ties_even() as usize) * 32;
    let height = (((h as f32 * ratio) as usize as f32 / 32.).round_ties_even() as usize) * 32;
    if width == 0 || height == 0 || width as u64 * height as u64 > MAX_DETECTION_PIXELS {
        return Err("Image aspect ratio exceeds the detection pixel budget".into());
    }
    Ok((width, height))
}

struct Token<'a> {
    text: &'a str,
    step: usize,
    score: f32,
}

fn decode<'a>(values: &[f32], classes: usize, dictionary: &[&'a str]) -> Vec<Token<'a>> {
    let mut previous = 0;
    let mut tokens = vec![];
    for (step, scores) in values.chunks_exact(classes).enumerate() {
        let (index, score) =
            scores
                .iter()
                .enumerate()
                .fold((0, f32::NEG_INFINITY), |best, (i, &p)| {
                    if p > best.1 { (i, p) } else { best }
                });
        if index != 0 && index != previous {
            tokens.push(Token {
                text: dictionary[index],
                step,
                score,
            });
        }
        previous = index;
    }
    tokens
}

fn word_spans(
    tokens: &[Token<'_>],
    q: Quad,
    steps: usize,
    padded_width: usize,
    width: usize,
) -> Vec<Word> {
    let groups = tokens
        .split(|t| t.text.chars().all(char::is_whitespace))
        .filter(|g| !g.is_empty())
        .collect::<Vec<_>>();
    let projection = Projection::new(q);
    groups
        .iter()
        .filter_map(|group| {
            let score = group.iter().map(|t| t.score).sum::<f32>() / group.len() as f32;
            if score < 0.5 {
                return None;
            }
            let polygon = if groups.len() == 1 {
                q
            } else {
                let scale = padded_width as f32 / (steps as f32 * width as f32);
                let left = ((group[0].step as f32 - 1.) * scale).clamp(0., 1.);
                let right = ((group.last().unwrap().step as f32 + 2.) * scale).clamp(0., 1.);
                if right <= left {
                    return None;
                }
                projection.strip(left, right)
            };
            Some(Word {
                text: group.iter().map(|t| t.text).collect(),
                polygon,
                score,
                source: Source::Detected,
                line: 0,
            })
        })
        .collect()
}

fn update_regions(regions: &mut Vec<Region>, words: &[Word]) {
    for region in regions.iter_mut() {
        region.misses += 1;
    }
    for word in words
        .iter()
        .filter(|w| w.source == Source::Detected && parse_number(&w.text).is_some())
    {
        let b = bounds(word.polygon);
        if let Some(region) = regions.iter_mut().find(|r| overlap(r.bounds, b)) {
            region.bounds = b;
            region.misses = 0;
        } else if regions.len() < 64 {
            regions.push(Region {
                bounds: b,
                misses: 0,
            });
        }
    }
    regions.retain(|r| r.misses <= 3);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn oversized_and_invalid_regions_do_not_discard_valid_neighbors() {
        let image = RgbImage::new(1200, 200);
        let good = rectangle([10., 10., 60., 24.]);
        let wide = rectangle([0., 50., 1100., 48.]);
        let invalid = rectangle([0., 0., 1., 1.]);
        let (crops, skipped) =
            prepare_crops(&image, &[good, wide, invalid, good], Source::Detected);
        assert_eq!(crops.len(), 2);
        assert!(crops.iter().all(|c| c.polygon == good && c.width == 120));
        assert_eq!(skipped.len(), 2);
        assert_eq!(skipped[0].polygon, wide);
        assert_eq!(skipped[0].source, Source::Detected);
        assert_eq!(
            skipped[0].reason,
            SkipReason::RecognitionWidth {
                required: 1100,
                maximum: 1024
            }
        );
        assert_eq!(skipped[1].reason, SkipReason::InvalidCrop);
    }

    #[test]
    fn recognition_width_budget_uses_rounded_up_scaled_and_rotated_crop() {
        let image = RgbImage::new(1100, 1100);
        let exact = rectangle([0., 0., 1024., 48.]);
        let beyond = rectangle([0., 0., 1025., 48.]);
        let fractional = rectangle([0., 0., 1003., 47.]); // 1024.34 rounds up, not down.
        let tall = rectangle([0., 0., 48., 1025.]);
        let (crops, skipped) = prepare_crops(
            &image,
            &[exact, beyond, fractional, tall],
            Source::Revisited,
        );
        assert_eq!(crops.len(), 1);
        assert_eq!(crops[0].width, 1024);
        assert_eq!(skipped.len(), 3);
        assert!(skipped.iter().all(|s| s.source == Source::Revisited
            && s.reason
                == SkipReason::RecognitionWidth {
                    required: 1025,
                    maximum: 1024
                }));
    }

    #[test]
    fn ctc_keeps_repeated_digits_separated_by_blank_and_whitespace() {
        let dictionary = ["", "1", " ", "Q"];
        let path = [3, 3, 0, 2, 1, 1, 0, 1];
        let probabilities = path
            .iter()
            .flat_map(|&p| (0..4).map(move |i| if i == p { 0.99 } else { 0.001 }))
            .collect::<Vec<_>>();
        let tokens = decode(&probabilities, 4, &dictionary);
        assert_eq!(tokens.iter().map(|t| t.text).collect::<String>(), "Q 11");
        let spans = word_spans(&tokens, rectangle([0., 0., 100., 20.]), 8, 100, 100);
        assert_eq!(
            spans.iter().map(|w| w.text.as_str()).collect::<Vec<_>>(),
            ["Q", "11"]
        );
        assert!(!overlap(bounds(spans[0].polygon), bounds(spans[1].polygon)));
    }
    #[test]
    fn recovery_never_seeds_or_refreshes_regions_and_geometry_does_not_accumulate() {
        let mut regions = vec![];
        let mut word = Word {
            text: "17".into(),
            polygon: rectangle([10., 10., 30., 20.]),
            score: 0.99,
            source: Source::Revisited,
            line: 0,
        };
        update_regions(&mut regions, &[word.clone()]);
        assert!(regions.is_empty());
        word.source = Source::Detected;
        update_regions(&mut regions, &[word.clone()]);
        word.polygon = rectangle([12., 12., 26., 18.]);
        update_regions(&mut regions, &[word.clone()]);
        assert_eq!(regions[0].bounds, [12., 12., 26., 18.]);
        word.source = Source::Revisited;
        for _ in 0..4 {
            update_regions(&mut regions, &[word.clone()]);
        }
        assert!(regions.is_empty());
    }
    #[test]
    fn pathological_image_shapes_are_rejected_before_allocation() {
        assert!(detection_size((0, 4)).is_err());
        assert!(detection_size((1, 2_000_000)).is_err());
        assert_eq!(detection_size((1280, 720)).unwrap(), (1312, 736));
        assert_eq!(detection_size((560, 260)).unwrap(), (1600, 736));
    }
}
