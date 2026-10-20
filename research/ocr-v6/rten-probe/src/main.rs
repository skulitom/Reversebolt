use rten_tensor::{NdTensor, prelude::*};
use serde::Deserialize;
use std::{collections::HashMap, fs::OpenOptions, io::Write, sync::Arc};

#[derive(Deserialize)]
struct Case {
    name: String,
    model: String,
    input: String,
    shape: [usize; 4],
    output: String,
    output_shape: Vec<usize>,
}

fn real_inputs(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let cases: Vec<Case> = serde_json::from_slice(&std::fs::read(path)?)?;
    let pool = Arc::new(rten::ThreadPool::with_num_threads(4));
    let mut models = HashMap::new();
    for case in cases {
        let count = case
            .shape
            .iter()
            .try_fold(1_usize, |a, &b| a.checked_mul(b));
        let Some(count) = count.filter(|&n| n > 0 && n <= 16_777_216) else {
            return Err("Input must contain 1..16777216 float values".into());
        };
        if std::fs::metadata(&case.input)?.len() != (count * 4) as u64 {
            return Err("Input length does not match shape".into());
        }
        let raw = std::fs::read(&case.input)?;
        let values: Vec<f32> = raw
            .as_chunks::<4>()
            .0
            .iter()
            .map(|bytes| f32::from_le_bytes(*bytes))
            .collect();
        if !values.iter().all(|v| v.is_finite()) {
            return Err("Nonfinite input".into());
        }
        if !models.contains_key(&case.model) {
            models.insert(case.model.clone(), rten::Model::load_file(&case.model)?);
        }
        let input = NdTensor::from_data(case.shape, values);
        let began = std::time::Instant::now();
        let output = models[&case.model].run_one(
            input.view().into(),
            Some(rten::RunOptions::default().with_thread_pool(Some(pool.clone()))),
        )?;
        let output: rten_tensor::Tensor<f32> = output.try_into()?;
        if output.shape() != case.output_shape {
            return Err("Output shape differs from reference runtime".into());
        }
        let elapsed = began.elapsed();
        let mut writer = std::io::BufWriter::new(
            OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&case.output)?,
        );
        for value in output.iter() {
            if !value.is_finite() {
                return Err("Nonfinite output".into());
            }
            writer.write_all(&value.to_le_bytes())?;
        }
        writer.flush()?;
        println!(
            "{}: {:?}, {} ms",
            case.name,
            output.shape(),
            elapsed.as_millis()
        );
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args().nth(1).as_deref() == Some("--cases") {
        return real_inputs(&std::env::args().nth(2).ok_or("Missing cases.json path")?);
    }
    for path in std::env::args().skip(1) {
        println!("Loading {path}");
        let model = rten::Model::load_file(&path)?;
        let [height, width] = if path.contains("_rec_") {
            [48, 320]
        } else {
            [96, 96]
        };
        let input = NdTensor::<f32, 4>::zeros([1, 3, height, width]);
        let began = std::time::Instant::now();
        let output = model.run_one(input.view().into(), None)?;
        let output: rten_tensor::Tensor<f32> = output.try_into()?;
        println!(
            "Output {:?}, elapsed {} ms",
            output.shape(),
            began.elapsed().as_millis()
        );
    }
    Ok(())
}
