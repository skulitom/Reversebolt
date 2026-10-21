//! Local neural OCR and observation-derived text regions.
//! No game names, HUD layouts, expected numbers or character substitutions.
use ocrs::{ImageSource, OcrEngine, OcrEngineParams, TextItem};
use reversebolt_core::{
    Result,
    live::{
        Screen, TextSource, Word, number_readings, parse_number, same_text_region, track_numbers,
    },
};
use rten_imageproc::{RectF, RotatedRect};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::Path;

pub struct Reader {
    engine: OcrEngine,
}

#[derive(Serialize)]
pub struct Reading {
    pub detected: Vec<Word>,
    pub revisited: Vec<Word>,
    pub elapsed_ms: u128,
}

impl Reader {
    pub fn load(directory: &Path) -> Result<Self> {
        for (name, size, expected) in [
            (
                "text-detection.onnx",
                2499479,
                "a917b23dbd9524b465df7e922641b3ff2981623df4ded5a0234004ef2fee7cfe",
            ),
            (
                "text-recognition.onnx",
                9713177,
                "86c145c2edb96c8caed5b1ebb8f44d706408922211451c309c625157dd6061c5",
            ),
        ] {
            let path = directory.join(name);
            if std::fs::metadata(&path)
                .map_err(|e| format!("{name}: {e}"))?
                .len()
                != size
            {
                return Err(format!("Unexpected model size: {name}"));
            }
            let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
            if format!("{:x}", Sha256::digest(&bytes)) != expected {
                return Err(format!("Model checksum mismatch: {name}"));
            }
        }
        let engine = OcrEngine::new(OcrEngineParams {
            detection_model: Some(
                rten::Model::load_file(directory.join("text-detection.onnx"))
                    .map_err(|e| e.to_string())?,
            ),
            recognition_model: Some(
                rten::Model::load_file(directory.join("text-recognition.onnx"))
                    .map_err(|e| e.to_string())?,
            ),
            ..Default::default()
        })
        .map_err(|e| e.to_string())?;
        Ok(Self { engine })
    }

    pub fn read(
        &self,
        image: &image::RgbImage,
        hints: &[[f32; 4]],
        windows: &[Word],
    ) -> Result<Reading> {
        if u64::from(image.width()) * u64::from(image.height()) > 2_100_000 {
            return Err("Neural text recognition supports up to 2.1 million pixels per capture in this preview.".into());
        }
        let start = std::time::Instant::now();
        let input = self
            .engine
            .prepare_input(
                ImageSource::from_bytes(image.as_raw(), image.dimensions())
                    .map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
        let boxes = self
            .engine
            .detect_words(&input)
            .map_err(|e| e.to_string())?;
        // Screens often place unrelated values on the same baseline, far apart.
        // Recognize detected words separately instead of treating a HUD row as prose.
        let lines: Vec<_> = boxes.iter().take(512).map(|rect| vec![*rect]).collect();
        let text = self
            .engine
            .recognize_text(&input, &lines)
            .map_err(|e| e.to_string())?;
        let mut detected = vec![];
        for (index, line) in text.iter().enumerate() {
            if let Some(line) = line {
                for word in line.words() {
                    let r = word.bounding_rect();
                    detected.push(Word {
                        text: word.to_string(),
                        bounds: [
                            r.left() as f32,
                            r.top() as f32,
                            r.width() as f32,
                            r.height() as f32,
                        ],
                        line: index,
                        source: TextSource::Ocrs,
                    });
                }
            }
        }

        let direct: Vec<_> = windows.iter().chain(&detected).cloned().collect();
        let agreed: Vec<_> = number_readings(&direct)
            .into_iter()
            .filter(|r| {
                r.value.is_some()
                    && r.alternatives
                        .iter()
                        .any(|w| w.source == TextSource::WindowsOcr)
                    && r.alternatives.iter().any(|w| w.source == TextSource::Ocrs)
            })
            .collect();
        // Region recognition is a recovery pass. Do not run an unnecessarily
        // constrained strip when both full-image readers already agree.
        let hints: Vec<_> = hints
            .iter()
            .take(64)
            .copied()
            .filter(|bounds| !agreed.iter().any(|r| same_text_region(r.bounds, *bounds)))
            .filter(|bounds| expanded_region(*bounds, image.dimensions()).is_some())
            .collect();
        let regions: Vec<_> = hints
            .iter()
            .map(|&bounds| {
                vec![RotatedRect::from_rect(
                    expanded_region(bounds, image.dimensions()).unwrap(),
                )]
            })
            .collect();
        let text = if regions.is_empty() {
            vec![]
        } else {
            self.engine
                .recognize_text(&input, &regions)
                .map_err(|e| e.to_string())?
        };
        let mut revisited = vec![];
        for (index, line) in text.iter().enumerate() {
            if let Some(line) = line {
                let value = line.to_string();
                // Keep unconstrained model output. Never force letters into digits.
                if numeric(&value) {
                    revisited.push(Word {
                        text: value,
                        // Recognition boxes cover the supplied strip, not a fresh
                        // detection. Never feed their expanded geometry back.
                        bounds: hints[index],
                        line: lines.len() + index,
                        source: TextSource::OcrsRegion,
                    });
                }
            }
        }
        Ok(Reading {
            detected,
            revisited,
            elapsed_ms: start.elapsed().as_millis(),
        })
    }
}

fn expanded_region(bounds: [f32; 4], size: (u32, u32)) -> Option<RectF> {
    let [x, y, w, h] = bounds;
    if !bounds.iter().all(|v| v.is_finite()) || w <= 0.0 || h <= 0.0 {
        return None;
    }
    let pad = (h * 0.25).clamp(3.0, 12.0);
    let left = (x - pad).max(0.0);
    let top = (y - pad).max(0.0);
    let right = (x + w + pad).min(size.0 as f32);
    let bottom = (y + h + pad).min(size.1 as f32);
    (right > left && bottom > top).then(|| RectF::from_tlhw(top, left, bottom - top, right - left))
}

fn numeric(text: &str) -> bool {
    parse_number(text).is_some()
}

#[derive(Clone)]
struct RememberedRegion {
    bounds: [f32; 4],
    misses: usize,
}

#[derive(Default)]
struct Regions {
    remembered: Vec<RememberedRegion>,
    size: Option<(u32, u32)>,
}

impl Regions {
    fn hints(&mut self, size: (u32, u32)) -> Vec<[f32; 4]> {
        if self.size != Some(size) {
            self.remembered.clear();
        }
        self.size = Some(size);
        self.remembered.iter().map(|r| r.bounds).collect()
    }

    fn update(&mut self, words: &[Word]) {
        for region in &mut self.remembered {
            region.misses += 1;
        }
        for word in words
            .iter()
            .filter(|w| w.source != TextSource::OcrsRegion && numeric(&w.text))
        {
            if let Some(region) = self
                .remembered
                .iter_mut()
                .find(|r| same_text_region(r.bounds, word.bounds))
            {
                // Geometry comes only from detections, never revisited strips.
                let left = region.bounds[0].min(word.bounds[0]);
                let right =
                    (region.bounds[0] + region.bounds[2]).max(word.bounds[0] + word.bounds[2]);
                let top = region.bounds[1].min(word.bounds[1]);
                let bottom =
                    (region.bounds[1] + region.bounds[3]).max(word.bounds[1] + word.bounds[3]);
                region.bounds = [left, top, right - left, bottom - top];
                region.misses = 0;
            } else if self.remembered.len() < 64 && same_text_region(word.bounds, word.bounds) {
                self.remembered.push(RememberedRegion {
                    bounds: word.bounds,
                    misses: 0,
                });
            }
        }
        self.remembered.retain(|r| r.misses <= 3);
    }
}

pub struct Observer {
    reader: Option<Reader>,
    paddle: Option<reversebolt_vision::Reader>,
    unavailable: Option<String>,
    regions: Regions,
}

impl Observer {
    pub fn load(models: &Path) -> Self {
        // A versioned model installation selects one neural reader. Retaining
        // obsolete model votes would manufacture disagreements between versions.
        if models.join("PP-OCRv6_medium_det_onnx").exists()
            || models.join("PP-OCRv6_medium_rec_onnx").exists()
        {
            let (paddle, unavailable) = match reversebolt_vision::Reader::load(models) {
                Ok(reader) => (Some(reader), None),
                Err(error) => (
                    None,
                    Some(format!(
                        "Using Windows text recognition only. PP-OCRv6 unavailable: {error}"
                    )),
                ),
            };
            return Self {
                reader: None,
                paddle,
                unavailable,
                regions: Regions::default(),
            };
        }
        let (reader, unavailable) = match Reader::load(models) {
            Ok(reader) => (Some(reader), None),
            Err(error) => (
                None,
                Some(format!(
                    "Using Windows text recognition only. Neural models unavailable: {error}"
                )),
            ),
        };
        Self {
            reader,
            paddle: None,
            unavailable,
            regions: Regions::default(),
        }
    }

    pub fn recognize(&mut self, screen: &mut Screen) -> Result<()> {
        let rgba = image::open(&screen.path)
            .map_err(|e| e.to_string())?
            .into_rgba8();
        let hints = self.regions.hints(rgba.dimensions());
        let mut warnings = vec![];
        match crate::native::ocr(&rgba) {
            Ok((text, words)) => {
                screen.text = text;
                screen.words = words;
            }
            Err(error) => warnings.push(format!("Windows text recognition: {error}")),
        }
        if let Some(reader) = &mut self.paddle {
            let rgb = image::DynamicImage::ImageRgba8(rgba).into_rgb8();
            match reader.read(&rgb) {
                Ok(reading) => {
                    if !reading.skipped_regions.is_empty() {
                        warnings.push(format!(
                            "PP-OCRv6 could not read {} text regions within its crop/width limits. Other regions were processed.",
                            reading.skipped_regions.len()
                        ));
                    }
                    for word in reading.words {
                        let left = word
                            .polygon
                            .iter()
                            .map(|p| p[0])
                            .fold(f32::INFINITY, f32::min);
                        let top = word
                            .polygon
                            .iter()
                            .map(|p| p[1])
                            .fold(f32::INFINITY, f32::min);
                        let right = word
                            .polygon
                            .iter()
                            .map(|p| p[0])
                            .fold(f32::NEG_INFINITY, f32::max);
                        let bottom = word
                            .polygon
                            .iter()
                            .map(|p| p[1])
                            .fold(f32::NEG_INFINITY, f32::max);
                        screen.words.push(Word {
                            text: word.text,
                            bounds: [left, top, right - left, bottom - top],
                            line: word.line,
                            source: match word.source {
                                reversebolt_vision::Source::Detected => TextSource::PpOcrV6,
                                reversebolt_vision::Source::Revisited => TextSource::PpOcrV6Region,
                            },
                        });
                    }
                }
                Err(error) => warnings.push(format!("PP-OCRv6 text recognition: {error}")),
            }
        } else if let Some(reader) = &self.reader {
            let rgb = image::DynamicImage::ImageRgba8(rgba).into_rgb8();
            match reader.read(&rgb, &hints, &screen.words) {
                Ok(reading) => {
                    screen.words.extend(reading.detected);
                    screen.words.extend(reading.revisited);
                }
                Err(error) => warnings.push(error),
            }
        } else if let Some(error) = &self.unavailable {
            warnings.push(error.clone());
        }
        if self.reader.is_some() {
            self.regions.update(&screen.words);
        }
        if warnings.is_empty() {
            Ok(())
        } else {
            Err(warnings.join(" "))
        }
    }
}

/// Reproducible offline evaluation, with regions learned only from earlier frames.
pub fn evaluate_sequence(models: &Path, frames: &Path, output: &Path) -> Result<()> {
    // Fail evaluation if the requested models are missing; production retains
    // Windows-only observations with a visible warning instead.
    if models.join("PP-OCRv6_medium_det_onnx").exists() {
        reversebolt_vision::Reader::load(models)?;
    } else {
        Reader::load(models)?;
    }
    let mut observer = Observer::load(models);
    let mut paths = std::fs::read_dir(frames)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|ext| ext == "png"))
        .collect::<Vec<_>>();
    paths.sort();
    let mut tracks: Vec<reversebolt_core::live::NumberTrack> = vec![];
    let mut report = vec![];
    for path in paths {
        let (width, height) = image::image_dimensions(&path).map_err(|e| e.to_string())?;
        let mut screen = Screen {
            path: path.to_string_lossy().into_owned(),
            width,
            height,
            text: String::new(),
            words: vec![],
            warning: None,
            method: "offline".into(),
        };
        if observer
            .regions
            .size
            .is_some_and(|size| size != (width, height))
        {
            for track in &mut tracks {
                track.active = false;
            }
        }
        let start = std::time::Instant::now();
        if let Err(error) = observer.recognize(&mut screen) {
            screen.warning = Some(error);
        }
        track_numbers(&mut tracks, &screen.words, report.len() + 1);
        report.push(
            serde_json::json!({"screen":screen,"elapsed_ms":start.elapsed().as_millis(),
            "tracks":tracks}),
        );
    }
    std::fs::write(
        output,
        serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognition_geometry_never_expands_or_keeps_its_own_seed_alive() {
        let mut regions = Regions::default();
        regions.hints((500, 500));
        let mut word = Word {
            text: "12".into(),
            bounds: [100.0, 100.0, 40.0, 20.0],
            line: 0,
            source: TextSource::WindowsOcr,
        };
        regions.update(&[word.clone()]);
        word.source = TextSource::OcrsRegion;
        word.bounds = [80.0, 80.0, 80.0, 60.0];
        for _ in 0..3 {
            regions.update(&[word.clone()]);
            assert_eq!(regions.hints((500, 500)), vec![[100.0, 100.0, 40.0, 20.0]]);
        }
        regions.update(&[word]);
        assert!(regions.hints((500, 500)).is_empty());
    }

    #[test]
    fn resize_resets_regions_and_invalid_boxes_are_rejected() {
        let mut regions = Regions::default();
        regions.hints((500, 500));
        regions.update(&[Word {
            text: "12".into(),
            bounds: [10.0, 20.0, 30.0, 20.0],
            line: 0,
            source: TextSource::Ocrs,
        }]);
        assert!(regions.hints((600, 500)).is_empty());
        assert!(expanded_region([f32::INFINITY, 0.0, 2.0, 2.0], (500, 500)).is_none());
        assert!(expanded_region([501.0, 0.0, 2.0, 2.0], (100, 100)).is_none());
    }
}
