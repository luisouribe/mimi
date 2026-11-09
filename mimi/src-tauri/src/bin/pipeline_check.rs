//! Tests Mimi's whole audio path with real recordings. It opens no window and no microphone.
//!
//! Usage: pipeline_check <audio folder> <languages> <gain> <lead silence seconds> <output.json>
//!        [parakeet|whisper-turbo|whisper-small] [clip|auto] [hint|no-hint] [gpu|cpu]

use mimi_lib::settings::ModelChoice;
use mimi_lib::{audio, engine, tidy};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Instant;

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
    cleanup_ms: f64,
    speech_seconds: f64,
    chunks: usize,
    reference: String,
    text: String,
    tidied: String,
    words: Vec<engine::WordTiming>,
}

#[derive(Serialize)]
struct Report {
    engine: String,
    model: String,
    load_seconds: f64,
    peak_memory_mb: f64,
    rate: u32,
    gain: f32,
    lead_silence: f32,
    first_words: Vec<engine::WordTiming>,
    clips: Vec<ClipResult>,
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

fn wait_until_ready(speech: &engine::Engine) -> f64 {
    loop {
        match speech.status() {
            engine::EngineStatus::Ready { load_seconds, problem, .. } => {
                assert!(problem.is_none(), "{problem:?}");
                return load_seconds;
            }
            engine::EngineStatus::Failed { message } => panic!("{message}"),
            engine::EngineStatus::Loading { .. } => std::thread::sleep(std::time::Duration::from_millis(50)),
        }
    }
}

fn main() {
    engine::prepare_graphics_environment();
    let args: Vec<String> = std::env::args().collect();
    assert!(args.len() >= 6, "usage: pipeline_check <audio> <langs> <gain> <lead silence> <output.json> [model] [clip|auto] [hint|no-hint] [gpu|cpu]");
    let root = PathBuf::from(&args[1]);
    let languages: Vec<&str> = args[2].split(',').collect();
    let gain: f32 = args[3].parse().unwrap();
    let lead: f32 = args[4].parse().unwrap();

    let choice = args.get(6).map_or("parakeet", String::as_str);
    let per_clip_language = args.get(7).map_or("clip", String::as_str) == "clip";
    let hint = args.get(8).map_or("hint", String::as_str) == "hint";
    let cpu_only = args.get(9).map_or("gpu", String::as_str) == "cpu";
    let model = match choice {
        "parakeet" => ModelChoice::Parakeet,
        "whisper-turbo" => ModelChoice::WhisperTurbo,
        "whisper-small" => ModelChoice::WhisperSmall,
        other => panic!("unknown model {other}"),
    };
    let options_for = |language: Option<String>| engine::EngineOptions {
        model,
        language,
        punctuation_hint: hint,
        cpu_only,
    };
    let data_folder = std::env::temp_dir().join(format!("mimi-check-{}", std::process::id()));
    let first_language = per_clip_language.then(|| languages[0].to_string());
    let speech = engine::Engine::start(options_for(first_language), data_folder.clone());
    let load_seconds = wait_until_ready(&speech);

    let manifest = std::fs::read_to_string(root.join("manifest.json")).unwrap();
    let clips: Vec<Clip> = serde_json::from_str(&manifest).unwrap();

    let mut results = Vec::new();
    let mut first_words = Vec::new();
    let mut rate_seen = 0;
    for clip in clips.iter().filter(|clip| languages.contains(&clip.lang.as_str())) {
        let (mut samples, rate) = audio::read_wav_mono(&root.join(&clip.file)).unwrap();
        rate_seen = rate;
        for sample in samples.iter_mut() {
            *sample *= gain;
        }
        if lead > 0.0 {
            let pad = vec![0.0; (rate as f32 * lead) as usize];
            let mut padded = pad.clone();
            padded.extend_from_slice(&samples);
            padded.extend_from_slice(&pad);
            samples = padded;
        }

        if per_clip_language {
            speech.configure(options_for(Some(clip.lang.clone()))).unwrap();
        }
        let started = Instant::now();
        let recognized = speech.recognize(samples, rate).unwrap();
        let elapsed = started.elapsed().as_secs_f64();
        let cleanup_ms = recognized.cleanup_ms;
        let speech_seconds = recognized.speech_seconds;
        let chunks = recognized.chunk_count;
        let transcript = recognized.transcript.unwrap_or(engine::Transcript { text: String::new(), words: Vec::new() });
        if first_words.is_empty() {
            first_words = transcript.words.iter().take(8).cloned().collect();
        }
        println!("{} {:.1}s of speech, cleanup {cleanup_ms:.0} ms, total {elapsed:.2}s", clip.file, clip.seconds);
        results.push(ClipResult {
            lang: clip.lang.clone(),
            file: clip.file.clone(),
            seconds: clip.seconds,
            elapsed,
            cleanup_ms,
            speech_seconds,
            chunks,
            reference: clip.reference.clone(),
            tidied: tidy::tidy(&transcript.text, per_clip_language.then_some(clip.lang.as_str())),
            text: transcript.text,
            words: transcript.words,
        });
    }

    let report = Report {
        engine: "mimi-pipeline".into(),
        model: choice.to_string(),
        load_seconds,
        peak_memory_mb: peak_memory_mb(),
        rate: rate_seen,
        gain,
        lead_silence: lead,
        first_words,
        clips: results,
    };
    std::fs::write(&args[5], serde_json::to_string_pretty(&report).unwrap()).unwrap();
    println!("peak memory {:.0} MB", report.peak_memory_mb);
}
