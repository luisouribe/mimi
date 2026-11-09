//! The speech engine. It keeps the chosen model loaded and ready on its own thread, so
//! dictating never waits for a model to load, and switching models never freezes the screen.

use crate::clean;
use crate::settings::ModelChoice;
use crate::vad::Vad;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{RecvTimeoutError, Sender, channel};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};
use transcribe_rs::onnx::Quantization;
use transcribe_rs::onnx::parakeet::{ParakeetModel, ParakeetParams, TimestampGranularity};
use transcribe_rs::whisper_cpp::{WhisperEngine, WhisperInferenceParams, WhisperLoadParams};

const PARAKEET_FOLDER: &str = "parakeet-tdt-0.6b-v3-int8";
const WHISPER_TURBO_FILE: &str = "whisper/ggml-large-v3-turbo-q5_0.bin";
const WHISPER_SMALL_FILE: &str = "whisper/ggml-small-q5_1.bin";
/// Exists only while Whisper first uses the graphics card. If Mimi finds it at startup,
/// the graphics card crashed Mimi last time.
const GPU_CHECK_FILE: &str = "gpu-check";
/// After this long without dictating, the model leaves memory. It returns when recording starts,
/// so it's usually ready again by the time the person stops talking.
const IDLE_RELEASE: Duration = Duration::from_secs(15 * 60);
const IDLE_RELEASE_TEST_SETTING: &str = "MIMI_IDLE_RELEASE_SECONDS";

#[derive(Debug, Clone, PartialEq)]
pub struct EngineOptions {
    pub model: ModelChoice,
    pub language: Option<String>,
    pub punctuation_hint: bool,
    pub cpu_only: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "state", rename_all = "lowercase")]
pub enum EngineStatus {
    Loading {
        model: ModelChoice,
    },
    Ready {
        model: ModelChoice,
        load_seconds: f64,
        uses_graphics_card: bool,
        problem: Option<String>,
    },
    Failed {
        message: String,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct WordTiming {
    pub text: String,
    pub start: f32,
    pub end: f32,
}

#[derive(Debug, Clone, Serialize)]
pub struct Transcript {
    pub text: String,
    pub words: Vec<WordTiming>,
}

/// The result of one dictation. `transcript` is `None` when no speech was heard.
#[derive(Debug, Clone, Serialize)]
pub struct Recognized {
    pub transcript: Option<Transcript>,
    pub speech_seconds: f64,
    pub chunk_count: usize,
    pub cleanup_ms: f64,
    pub recognition_ms: f64,
}

/// A model in the Settings list.
#[derive(Debug, Clone, Serialize)]
pub struct ModelInfo {
    pub id: ModelChoice,
    pub name: &'static str,
    pub description: &'static str,
    pub installed: bool,
}

enum Job {
    Recognize {
        mono: Vec<f32>,
        rate: u32,
        reply: Sender<Result<Recognized, String>>,
    },
    Configure(EngineOptions),
    /// Recording has started, so a released model should load again.
    Wake,
}

pub struct Engine {
    jobs: Sender<Job>,
    status: Arc<Mutex<EngineStatus>>,
}

enum Recognizer {
    Parakeet(Box<ParakeetModel>),
    Whisper(Box<WhisperEngine>),
}

/// A short sample in the chosen language. Whisper imitates the style of the text it's shown,
/// so this nudges it to punctuate. It's never added to the result.
fn punctuation_hint(language: &str) -> Option<&'static str> {
    Some(match language {
        "es" => "Hola, ¿cómo estás? Muy bien, gracias. Nos vemos mañana.",
        "en" => "Hello, how are you? Very well, thank you. See you tomorrow.",
        "ja" => "こんにちは、お元気ですか。はい、元気です。また明日。",
        "pt" => "Olá, como vai? Muito bem, obrigado. Até amanhã.",
        "fr" => "Bonjour, comment allez-vous ? Très bien, merci. À demain.",
        "de" => "Hallo, wie geht es dir? Sehr gut, danke. Bis morgen.",
        "it" => "Ciao, come stai? Molto bene, grazie. A domani.",
        "zh" => "你好,你好吗?我很好,谢谢。明天见。",
        "ko" => "안녕하세요, 잘 지내세요? 네, 잘 지내요. 내일 봐요.",
        _ => return None,
    })
}

/// Whisper sometimes writes captions such as "[Music]" for sounds that aren't speech.
pub fn is_only_a_sound_caption(text: &str) -> bool {
    let trimmed = text.trim();
    let bracketed = (trimmed.starts_with('[') && trimmed.ends_with(']'))
        || (trimmed.starts_with('(') && trimmed.ends_with(')'))
        || trimmed.chars().all(|c| c == '♪' || c == '♫' || c.is_whitespace());
    bracketed && trimmed.matches(['[', '(']).count() <= 1
}

/// Languages written without spaces between words, such as Japanese and Chinese.
fn joins_without_space(c: char) -> bool {
    matches!(c,
        '\u{3000}'..='\u{30FF}' | '\u{3400}'..='\u{4DBF}' | '\u{4E00}'..='\u{9FFF}' | '\u{FF00}'..='\u{FFEF}')
}

/// Joins the pieces of a long dictation.
pub fn join_pieces(pieces: &[String]) -> String {
    let mut joined = String::new();
    for piece in pieces.iter().map(|piece| piece.trim()).filter(|piece| !piece.is_empty()) {
        let tight = joined.chars().last().is_some_and(joins_without_space)
            && piece.chars().next().is_some_and(joins_without_space);
        if !joined.is_empty() && !tight {
            joined.push(' ');
        }
        joined.push_str(piece);
    }
    joined
}

impl Recognizer {
    fn transcribe(&mut self, samples: &[f32], options: &EngineOptions) -> Result<Transcript, String> {
        match self {
            Recognizer::Parakeet(model) => {
                let params = ParakeetParams {
                    timestamp_granularity: Some(TimestampGranularity::Word),
                    ..Default::default()
                };
                let result = model.transcribe_with(samples, &params).map_err(|error| error.to_string())?;
                let words = result
                    .segments
                    .unwrap_or_default()
                    .into_iter()
                    .map(|segment| WordTiming {
                        text: segment.text.trim().to_string(),
                        start: segment.start,
                        end: segment.end,
                    })
                    .collect();
                Ok(Transcript {
                    text: result.text.trim().to_string(),
                    words,
                })
            }
            Recognizer::Whisper(engine) => {
                let hint = options.language.as_deref().filter(|_| options.punctuation_hint).and_then(punctuation_hint);
                let params = WhisperInferenceParams {
                    language: options.language.clone(),
                    initial_prompt: hint.map(str::to_string),
                    ..Default::default()
                };
                let result = engine.transcribe_with(samples, &params).map_err(|error| error.to_string())?;
                let text = result.text.trim().to_string();
                Ok(Transcript {
                    text: if is_only_a_sound_caption(&text) { String::new() } else { text },
                    words: Vec::new(),
                })
            }
        }
    }
}

/// Cleans the audio, recognizes each piece in order, and joins the results.
fn recognize(
    recognizer: &mut Recognizer,
    options: &EngineOptions,
    vad: &mut Vad,
    mono: &[f32],
    rate: u32,
) -> Result<Recognized, String> {
    let cleanup_started = Instant::now();
    let prepared = clean::prepare(mono, rate, vad)?;
    let cleanup_ms = cleanup_started.elapsed().as_secs_f64() * 1000.0;

    let Some(prepared) = prepared else {
        return Ok(Recognized {
            transcript: None,
            speech_seconds: 0.0,
            chunk_count: 0,
            cleanup_ms,
            recognition_ms: 0.0,
        });
    };

    let recognition_started = Instant::now();
    let mut pieces = Vec::with_capacity(prepared.chunks.len());
    let mut words = Vec::new();
    let mut offset = 0.0f32;
    for chunk in &prepared.chunks {
        let part = recognizer.transcribe(chunk, options)?;
        words.extend(part.words.into_iter().map(|word| WordTiming {
            start: word.start + offset,
            end: word.end + offset,
            text: word.text,
        }));
        pieces.push(part.text);
        offset += chunk.len() as f32 / clean::TARGET_RATE as f32;
    }
    Ok(Recognized {
        transcript: Some(Transcript {
            text: join_pieces(&pieces),
            words,
        }),
        speech_seconds: prepared.speech_seconds,
        chunk_count: prepared.chunks.len(),
        cleanup_ms,
        recognition_ms: recognition_started.elapsed().as_secs_f64() * 1000.0,
    })
}

/// Loads a model and runs it once, so the first real dictation is as fast as the rest.
/// For Whisper on a graphics card, this is the slow step that prepares the card.
fn load_recognizer(options: &EngineOptions, data_folder: &Path) -> Result<(Recognizer, bool), String> {
    let Some(path) = model_path(options.model) else {
        return Err(format!("The {} model isn't installed.", model_name(options.model)));
    };
    // Silence, because Whisper can spend minutes trying to read a test tone as words.
    let warm_audio = vec![0.0f32; 16_000];

    match options.model {
        ModelChoice::Parakeet => {
            let model = ParakeetModel::load(&path, &Quantization::Int8).map_err(|error| error.to_string())?;
            let mut recognizer = Recognizer::Parakeet(Box::new(model));
            recognizer.transcribe(&warm_audio, options)?;
            Ok((recognizer, false))
        }
        ModelChoice::WhisperTurbo | ModelChoice::WhisperSmall => {
            let use_gpu = !options.cpu_only;
            let check_file = data_folder.join(GPU_CHECK_FILE);
            if use_gpu {
                let _ = std::fs::create_dir_all(data_folder);
                let _ = std::fs::write(&check_file, b"");
            }
            let params = WhisperLoadParams {
                use_gpu,
                ..Default::default()
            };
            let result = WhisperEngine::load_with_params(&path, params)
                .map_err(|error| error.to_string())
                .and_then(|engine| {
                    let mut recognizer = Recognizer::Whisper(Box::new(engine));
                    recognizer.transcribe(&warm_audio, options)?;
                    Ok(recognizer)
                });
            let _ = std::fs::remove_file(&check_file);
            result.map(|recognizer| (recognizer, use_gpu))
        }
    }
}

impl Engine {
    /// Starts loading in the background and returns immediately.
    pub fn start(options: EngineOptions, data_folder: PathBuf) -> Self {
        let (jobs, inbox) = channel::<Job>();
        let status = Arc::new(Mutex::new(EngineStatus::Loading { model: options.model }));
        let shared = status.clone();
        let set_status = move |next: EngineStatus| {
            *shared.lock().unwrap_or_else(PoisonError::into_inner) = next;
        };
        thread::Builder::new()
            .name("mimi-speech".into())
            .spawn(move || {
                let mut vad = match Vad::new() {
                    Ok(vad) => vad,
                    Err(error) => {
                        let message = format!("Speech detection couldn't load: {error}");
                        set_status(EngineStatus::Failed { message: message.clone() });
                        for job in inbox {
                            if let Job::Recognize { reply, .. } = job {
                                let _ = reply.send(Err(message.clone()));
                            }
                        }
                        return;
                    }
                };

                let mut current = options;
                let started = Instant::now();
                let mut active: Option<(Recognizer, bool)> = None;
                let mut problem = None;
                match load_recognizer(&current, &data_folder) {
                    Ok(loaded) => active = Some(loaded),
                    Err(error) => problem = Some(error),
                }
                // If the chosen model can't load, Spanish and English should still work.
                if active.is_none() && current.model != ModelChoice::Parakeet {
                    current.model = ModelChoice::Parakeet;
                    match load_recognizer(&current, &data_folder) {
                        Ok(loaded) => active = Some(loaded),
                        Err(error) => problem = Some(error),
                    }
                }
                let report = |active: &Option<(Recognizer, bool)>, model, seconds, problem: Option<String>| {
                    set_status(match active {
                        Some((_, gpu)) => EngineStatus::Ready {
                            model,
                            load_seconds: seconds,
                            uses_graphics_card: *gpu,
                            problem,
                        },
                        None => EngineStatus::Failed {
                            message: format!(
                                "The speech model couldn't load: {}",
                                problem.unwrap_or_else(|| "unknown problem".into())
                            ),
                        },
                    })
                };
                report(&active, current.model, started.elapsed().as_secs_f64(), problem);

                let idle_release = idle_release_time();
                // True while the model is out of memory only because it wasn't being used.
                let mut released = false;
                loop {
                    let job = if active.is_some() {
                        match inbox.recv_timeout(idle_release) {
                            Ok(job) => job,
                            Err(RecvTimeoutError::Timeout) => {
                                active = None;
                                released = true;
                                continue;
                            }
                            Err(RecvTimeoutError::Disconnected) => break,
                        }
                    } else {
                        match inbox.recv() {
                            Ok(job) => job,
                            Err(_) => break,
                        }
                    };
                    let reload_problem = if released && matches!(job, Job::Recognize { .. } | Job::Wake) {
                        released = false;
                        match load_recognizer(&current, &data_folder) {
                            Ok(loaded) => {
                                active = Some(loaded);
                                None
                            }
                            Err(error) => Some(format!("{} couldn't load: {error}", model_name(current.model))),
                        }
                    } else {
                        None
                    };
                    match job {
                        Job::Wake => {
                            if let Some(message) = reload_problem {
                                report(&active, current.model, 0.0, Some(message));
                            }
                        }
                        Job::Recognize { mono, rate, reply } => {
                            let result = match (&mut active, reload_problem) {
                                (_, Some(message)) => Err(message),
                                (Some((recognizer, _)), None) => recognize(recognizer, &current, &mut vad, &mono, rate),
                                (None, None) => Err("No speech model is loaded.".to_string()),
                            };
                            let _ = reply.send(result);
                        }
                        Job::Configure(next) => {
                            // A language change on a released model waits for the next dictation.
                            let same_model = (active.is_some() || released) && next.model == current.model && next.cpu_only == current.cpu_only;
                            if same_model {
                                current = next;
                                continue;
                            }
                            let started = Instant::now();
                            // The new model loads first, so a failure leaves the old one working.
                            match load_recognizer(&next, &data_folder) {
                                Ok(loaded) => {
                                    active = Some(loaded);
                                    released = false;
                                    current = next;
                                    report(&active, current.model, started.elapsed().as_secs_f64(), None);
                                }
                                Err(error) => {
                                    let message = format!("{} couldn't load: {error}", model_name(next.model));
                                    report(&active, current.model, started.elapsed().as_secs_f64(), Some(message));
                                }
                            }
                        }
                    }
                }
            })
            .expect("speech thread failed to start");
        Self { jobs, status }
    }

    pub fn status(&self) -> EngineStatus {
        self.status.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    /// Changes the language right away, or loads a different model in the background.
    pub fn configure(&self, options: EngineOptions) -> Result<(), String> {
        if let EngineStatus::Ready { model, .. } = self.status()
            && model != options.model
        {
            *self.status.lock().unwrap_or_else(PoisonError::into_inner) = EngineStatus::Loading { model: options.model };
        }
        self.jobs.send(Job::Configure(options)).map_err(|error| error.to_string())
    }

    /// Brings a released model back. Recording calls this, so the wait happens while the person talks.
    pub fn wake(&self) {
        let _ = self.jobs.send(Job::Wake);
    }

    /// Waits for the model if it's still loading, then cleans and recognizes the audio.
    pub fn recognize(&self, mono: Vec<f32>, rate: u32) -> Result<Recognized, String> {
        let (reply, answer) = channel();
        self.jobs
            .send(Job::Recognize { mono, rate, reply })
            .map_err(|error| error.to_string())?;
        answer.recv().map_err(|error| error.to_string())?
    }
}

fn idle_release_time() -> Duration {
    std::env::var(IDLE_RELEASE_TEST_SETTING)
        .ok()
        .and_then(|seconds| seconds.parse().ok())
        .map_or(IDLE_RELEASE, Duration::from_secs)
}

fn model_places() -> Vec<PathBuf> {
    let mut places: Vec<PathBuf> = Vec::new();
    if let Some(custom) = std::env::var_os("MIMI_MODELS_DIR") {
        places.push(PathBuf::from(custom));
    }
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        places.push(Path::new(&local).join("Mimi").join("models"));
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(folder) = exe.parent()
    {
        places.push(folder.join("models"));
    }
    places.push(Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join("models"));
    places
}

/// Finds a model on this PC. Downloading models arrives with the installer.
fn model_path(model: ModelChoice) -> Option<PathBuf> {
    model_places().into_iter().find_map(|place| match model {
        ModelChoice::Parakeet => {
            let folder = place.join(PARAKEET_FOLDER);
            folder.join("vocab.txt").is_file().then_some(folder)
        }
        ModelChoice::WhisperTurbo => {
            let file = place.join(WHISPER_TURBO_FILE);
            file.is_file().then_some(file)
        }
        ModelChoice::WhisperSmall => {
            let file = place.join(WHISPER_SMALL_FILE);
            file.is_file().then_some(file)
        }
    })
}

fn model_name(model: ModelChoice) -> &'static str {
    match model {
        ModelChoice::Parakeet => "Parakeet V3",
        ModelChoice::WhisperTurbo => "Whisper Turbo",
        ModelChoice::WhisperSmall => "Whisper Small",
    }
}

/// The choices shown in Settings.
pub fn list_models() -> Vec<ModelInfo> {
    [
        (ModelChoice::Parakeet, "25 European languages, Spanish and English included. Fastest, and no graphics card needed."),
        (ModelChoice::WhisperTurbo, "100 languages, Japanese included. The most accurate Whisper."),
        (ModelChoice::WhisperSmall, "99 languages, Japanese included. Lighter, but less accurate."),
    ]
    .into_iter()
    .map(|(id, description)| ModelInfo {
        id,
        name: model_name(id),
        description,
        installed: model_path(id).is_some(),
    })
    .collect()
}

/// True if the graphics card crashed Mimi during its last check. This also clears the mark.
pub fn graphics_card_crashed_last_time(data_folder: &Path) -> bool {
    let check_file = data_folder.join(GPU_CHECK_FILE);
    let crashed = check_file.is_file();
    if crashed {
        let _ = std::fs::remove_file(check_file);
    }
    crashed
}

/// Quitting during a check isn't a crash, so the mark is cleared on the way out.
pub fn clear_graphics_card_check(data_folder: &Path) {
    let _ = std::fs::remove_file(data_folder.join(GPU_CHECK_FILE));
}

/// Screen recorders and overlays can inject code into graphics programs and crash them.
/// This asks Windows' graphics loader to skip them. It must run before any threads start.
pub fn prepare_graphics_environment() {
    unsafe {
        std::env::set_var("VK_LOADER_LAYERS_DISABLE", "~implicit~");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pieces(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| item.to_string()).collect()
    }

    #[test]
    fn spanish_and_english_pieces_are_joined_with_a_space() {
        assert_eq!(join_pieces(&pieces(&["Hola.", "¿Cómo estás?"])), "Hola. ¿Cómo estás?");
    }

    #[test]
    fn japanese_pieces_are_joined_without_a_space() {
        assert_eq!(join_pieces(&pieces(&["今日は晴れです。", "明日は雨です。"])), "今日は晴れです。明日は雨です。");
    }

    #[test]
    fn mixed_pieces_keep_a_space_at_the_language_change() {
        assert_eq!(join_pieces(&pieces(&["今日は", "OK"])), "今日は OK");
    }

    #[test]
    fn empty_pieces_leave_no_extra_spaces() {
        assert_eq!(join_pieces(&pieces(&["", " Hola ", "", "mundo"])), "Hola mundo");
        assert_eq!(join_pieces(&pieces(&["", " "])), "");
    }

    #[test]
    fn sound_captions_are_not_treated_as_speech() {
        for caption in ["[Music]", "(música)", "[BLANK_AUDIO]", " ♪♪ "] {
            assert!(is_only_a_sound_caption(caption), "{caption}");
        }
    }

    #[test]
    fn real_sentences_are_kept_even_with_brackets_inside() {
        for sentence in ["Hola mundo.", "[Nota] revisar esto", "(uno) y (dos)", "今日は晴れです。"] {
            assert!(!is_only_a_sound_caption(sentence), "{sentence}");
        }
    }

    #[test]
    fn main_languages_have_a_punctuation_hint_and_others_are_safe() {
        for language in ["es", "en", "ja"] {
            assert!(punctuation_hint(language).is_some(), "{language}");
        }
        assert_eq!(punctuation_hint("xx"), None);
    }

    #[test]
    fn a_crash_mark_is_found_once_and_cleared() {
        let folder = std::env::temp_dir().join(format!("mimi-engine-check-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        assert!(!graphics_card_crashed_last_time(&folder));
        std::fs::write(folder.join(GPU_CHECK_FILE), b"").unwrap();
        assert!(graphics_card_crashed_last_time(&folder));
        assert!(!graphics_card_crashed_last_time(&folder));
        std::fs::remove_dir_all(&folder).unwrap();
    }
}
