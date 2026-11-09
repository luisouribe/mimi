//! Measures how fast each speech model runs on this PC.
//!
//! Usage: mimi-bench <parakeet|whisper> <model path> <test-audio folder> <languages> <output.json>
//! Example languages value: en,es

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Instant;
use transcribe_rs::onnx::parakeet::{ParakeetModel, ParakeetParams, TimestampGranularity};
use transcribe_rs::onnx::Quantization;
use transcribe_rs::whisper_cpp::{WhisperEngine, WhisperInferenceParams};

#[derive(Deserialize)]
struct Clip {
    lang: String,
    file: String,
    seconds: f64,
    reference: String,
}

#[derive(Serialize)]
struct ClipResult {
    lang: String,
    file: String,
    seconds: f64,
    elapsed: f64,
    reference: String,
    text: String,
}

#[derive(Serialize)]
struct Report {
    engine: String,
    model: String,
    load_seconds: f64,
    peak_memory_mb: f64,
    clips: Vec<ClipResult>,
}

enum Engine {
    Parakeet(ParakeetModel),
    Whisper(WhisperEngine),
}

impl Engine {
    fn transcribe(&mut self, samples: &[f32], lang: &str) -> Result<String> {
        match self {
            Engine::Parakeet(model) => {
                let params = ParakeetParams {
                    timestamp_granularity: Some(TimestampGranularity::Segment),
                    ..Default::default()
                };
                let result = model
                    .transcribe_with(samples, &params)
                    .map_err(|error| anyhow::anyhow!("{error}"))?;
                Ok(result.text)
            }
            Engine::Whisper(engine) => {
                let params = WhisperInferenceParams {
                    language: Some(lang.to_string()),
                    ..Default::default()
                };
                let result = engine
                    .transcribe_with(samples, &params)
                    .map_err(|error| anyhow::anyhow!("{error}"))?;
                Ok(result.text)
            }
        }
    }
}

fn peak_memory_mb() -> f64 {
    use windows_sys::Win32::System::ProcessStatus::{GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS};
    use windows_sys::Win32::System::Threading::GetCurrentProcess;
    unsafe {
        let mut counters: PROCESS_MEMORY_COUNTERS = std::mem::zeroed();
        counters.cb = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
        if GetProcessMemoryInfo(GetCurrentProcess(), &mut counters, counters.cb) == 0 {
            return 0.0;
        }
        counters.PeakWorkingSetSize as f64 / 1_048_576.0
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 6 {
        bail!("usage: mimi-bench <parakeet|whisper> <model> <test-audio> <langs> <output.json>");
    }
    let (kind, model_path, audio_root, langs, output) =
        (&args[1], PathBuf::from(&args[2]), PathBuf::from(&args[3]), &args[4], &args[5]);
    let wanted: Vec<&str> = langs.split(',').collect();

    let manifest = std::fs::read_to_string(audio_root.join("manifest.json"))
        .context("reading manifest")?;
    let clips: Vec<Clip> = serde_json::from_str(&manifest)?;

    let started = Instant::now();
    let mut engine = match kind.as_str() {
        "parakeet" => Engine::Parakeet(
            ParakeetModel::load(&model_path, &Quantization::Int8)
                .map_err(|error| anyhow::anyhow!("{error}"))?,
        ),
        "whisper" => Engine::Whisper(
            WhisperEngine::load(&model_path).map_err(|error| anyhow::anyhow!("{error}"))?,
        ),
        other => bail!("unknown engine {other}"),
    };
    let load_seconds = started.elapsed().as_secs_f64();
    println!("loaded {} in {load_seconds:.2}s", model_path.display());

    let mut results = Vec::new();
    for clip in clips.iter().filter(|clip| wanted.contains(&clip.lang.as_str())) {
        let path: &Path = &audio_root.join(&clip.file);
        let samples = transcribe_rs::audio::read_wav_samples(path)
            .map_err(|error| anyhow::anyhow!("{error}"))?;
        let started = Instant::now();
        let text = engine.transcribe(&samples, &clip.lang)?;
        let elapsed = started.elapsed().as_secs_f64();
        println!("{} {:.1}s audio in {elapsed:.2}s", clip.file, clip.seconds);
        results.push(ClipResult {
            lang: clip.lang.clone(),
            file: clip.file.clone(),
            seconds: clip.seconds,
            elapsed,
            reference: clip.reference.clone(),
            text: text.trim().to_string(),
        });
    }

    let report = Report {
        engine: kind.clone(),
        model: model_path.display().to_string(),
        load_seconds,
        peak_memory_mb: peak_memory_mb(),
        clips: results,
    };
    std::fs::write(output, serde_json::to_string_pretty(&report)?)?;
    println!("peak memory {:.0} MB", report.peak_memory_mb);
    Ok(())
}
