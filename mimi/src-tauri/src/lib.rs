//! Connects the shortcut, microphone, speech engine, pasting, history and screens.

pub mod audio;
#[cfg(windows)]
pub mod autostart;
#[cfg(not(windows))]
#[path = "mac/autostart.rs"]
pub mod autostart;
pub mod clean;
pub mod diag;
pub mod engine;
pub mod history;
pub mod hotkey;
#[cfg(windows)]
pub mod indicator;
#[cfg(not(windows))]
#[path = "mac/indicator.rs"]
pub mod indicator;
pub mod lists;
#[cfg(windows)]
pub mod paste;
#[cfg(not(windows))]
#[path = "mac/paste.rs"]
pub mod paste;
pub mod quiet;
pub mod settings;
pub mod snippets;
pub mod sounds;
pub mod tidy;
pub mod vad;

use audio::{Capture, MicInfo, Microphone};
use engine::{Engine, EngineOptions, EngineStatus, ModelInfo};
use history::{History, HistoryEntry, NewEntry};
use hotkey::{HotkeyEvent, Shortcut};
use paste::{PasteOutcome, Target};
use serde::Serialize;
use settings::{ModelChoice, Settings};
use snippets::Snippet;
use sounds::Chime;
use std::io::Write;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::channel;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, RunEvent, State, WindowEvent};

const HISTORY_PAGE: usize = 100;
const TRAY_ID: &str = "mimi";
const HISTORY_SEARCH_LIMIT: usize = 200;
/// How long a short message stays on the indicator.
const NOTICE_TIME: Duration = Duration::from_millis(2_600);
const COPIED_NOTICE: &str = if cfg!(target_os = "macos") { "COPIED: PRESS CMD + V" } else { "COPIED: PRESS CTRL + V" };

struct Session {
    capture: Arc<Capture>,
    stop_levels: Arc<AtomicBool>,
    mic_ready_ms: Arc<Mutex<Option<f64>>>,
    shortcut_ms: Option<f64>,
}

struct AppState {
    microphone: Microphone,
    engine: Engine,
    settings: Mutex<Settings>,
    data_folder: PathBuf,
    history: Result<History, String>,
    session: Mutex<Option<Session>>,
    /// True from the moment recording stops until the text has been delivered.
    busy: AtomicBool,
    chosen_microphone: Mutex<Option<String>>,
    timings_file: Option<PathBuf>,
    quieter: quiet::Quieter,
}

#[derive(Debug, Clone, Serialize)]
struct Timings {
    shortcut_to_recording_ms: Option<f64>,
    mic_ready_ms: Option<f64>,
    recorded_seconds: f64,
    speech_seconds: f64,
    chunk_count: usize,
    cleanup_ms: f64,
    recognition_ms: f64,
    tidy_ms: f64,
    stop_to_text_ms: f64,
    paste_ms: f64,
    total_ms: f64,
    outcome: String,
}

#[derive(Debug, Clone, Serialize)]
struct Dictation {
    text: String,
    heard_speech: bool,
    word_count: usize,
    outcome: Option<PasteOutcome>,
    entry: Option<HistoryEntry>,
    timings: Timings,
}

/// The current stage, sent to both the main window and the indicator.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "phase", rename_all = "snake_case")]
enum Stage {
    Idle,
    Opening,
    Listening,
    Working,
    Notice { message: String },
}

fn announce(app: &AppHandle, stage: Stage) {
    let _ = app.emit("dictation-stage", stage);
}

/// False when Windows started Mimi at sign-in, so her first screen stays in the tray.
static SHOW_WHEN_READY: AtomicBool = AtomicBool::new(true);

fn is_self_test() -> bool {
    std::env::var_os("MIMI_SELF_TEST_OUT").is_some()
}

/// Starts listening. `shortcut_at` is when the shortcut was pressed, for measuring speed.
fn begin(app: &AppHandle, device: Option<String>, shortcut_at: Option<Instant>) -> Result<(), String> {
    let state = app.state::<AppState>();
    diag::note(format!(
        "start asked by {} while {} is active",
        if shortcut_at.is_some() { "shortcut" } else { "button" },
        Target::current().app_name()
    ));
    if state.busy.load(Ordering::Acquire) {
        return Err("Mimi is still typing the last dictation.".into());
    }
    state.engine.wake();
    let mut session = state.session.lock().map_err(|_| "Mimi hit an internal error.".to_string())?;
    if session.is_some() {
        return Err("Mimi is already listening.".into());
    }
    let device = device.or_else(|| state.chosen_microphone.lock().ok().and_then(|chosen| chosen.clone()));
    let (chimes, quiet_other_apps) = listening_sounds(&state);
    start_sounds(&state, chimes, quiet_other_apps);

    // Show the indicator first, because opening a microphone can take most of a second.
    if !is_self_test() {
        indicator::show(app);
    }
    announce(app, Stage::Opening);
    let started = Instant::now();
    let capture = match state.microphone.start(device) {
        Ok(capture) => capture,
        Err(message) => {
            drop(session);
            state.quieter.restore();
            notify_then_idle(app, "MIC PROBLEM");
            return Err(message);
        }
    };
    let stop_levels = Arc::new(AtomicBool::new(false));
    let mic_ready_ms = Arc::new(Mutex::new(None));

    let (level_capture, level_stop, level_ready, level_app) =
        (capture.clone(), stop_levels.clone(), mic_ready_ms.clone(), app.clone());
    thread::spawn(move || {
        let mut announced = false;
        while !level_stop.load(Ordering::Relaxed) {
            if !announced && level_capture.is_ready() {
                announced = true;
                if let Ok(mut ready) = level_ready.lock() {
                    *ready = Some(started.elapsed().as_secs_f64() * 1000.0);
                }
                announce(&level_app, Stage::Listening);
            }
            if announced {
                let _ = level_app.emit("mic-level", level_capture.level());
            }
            thread::sleep(Duration::from_millis(50));
        }
    });

    *session = Some(Session {
        capture,
        stop_levels,
        mic_ready_ms,
        shortcut_ms: shortcut_at.map(|at| at.elapsed().as_secs_f64() * 1000.0),
    });
    Ok(())
}

fn finish(app: &AppHandle) -> Result<Dictation, String> {
    let stopped = Instant::now();
    // The app active now is where the words belong, even if focus moves while they're processed.
    let target = Target::current();
    let state = app.state::<AppState>();
    diag::note(format!("stop asked while {} is active", target.app_name()));
    let session = state
        .session
        .lock()
        .map_err(|_| "Mimi hit an internal error.".to_string())?
        .take()
        .ok_or_else(|| "Mimi isn't listening.".to_string())?;
    state.busy.store(true, Ordering::Release);
    announce(app, Stage::Working);

    let result = process(app, &state, session, target, stopped);

    state.busy.store(false, Ordering::Release);
    hotkey::set_recording(false);
    match &result {
        Ok(dictation) => match &dictation.outcome {
            Some(PasteOutcome::CopiedInstead { .. }) => notify_then_idle(app, COPIED_NOTICE),
            _ if !dictation.heard_speech => notify_then_idle(app, "NO SPEECH HEARD"),
            _ => go_idle(app),
        },
        Err(_) => notify_then_idle(app, "MIMI HIT A PROBLEM"),
    }
    result
}

fn process(
    app: &AppHandle,
    state: &AppState,
    session: Session,
    target: Target,
    stopped: Instant,
) -> Result<Dictation, String> {
    state.microphone.stop();
    session.stop_levels.store(true, Ordering::Relaxed);
    // Played only once the microphone is closed, so the chime isn't recorded.
    let (chimes, quiet_other_apps) = listening_sounds(state);
    stop_sounds(state, chimes, quiet_other_apps);

    let samples = session.capture.take_samples();
    let rate = session.capture.rate();
    if samples.is_empty()
        && let Some(message) = session.capture.failure()
    {
        return Err(message);
    }
    let recorded_seconds = samples.len() as f64 / rate as f64;

    let loudest = samples.iter().fold(0.0f32, |peak, sample| peak.max(sample.abs()));
    diag::note(format!("recorded {recorded_seconds:.2} s, loudest sound {loudest:.4}"));
    let recognized = state.engine.recognize(samples, rate)?;
    let tidy_started = Instant::now();
    let raw_text = recognized.transcript.map_or(String::new(), |transcript| transcript.text);
    let text = tidied(&raw_text, state)?;
    diag::note(format!(
        "heard {:.2} s of speech, {} characters recognized, {} after cleanup",
        recognized.speech_seconds,
        raw_text.chars().count(),
        text.chars().count()
    ));
    let tidy_ms = tidy_started.elapsed().as_secs_f64() * 1000.0;
    let stop_to_text_ms = stopped.elapsed().as_secs_f64() * 1000.0;

    let heard_speech = !text.is_empty();
    // A dictation that was only a line break command is pasted, but has nothing to keep in the history.
    let has_words = !text.trim().is_empty();
    let word_count = text.split_whitespace().count();

    let paste_started = Instant::now();
    let outcome = if heard_speech && !is_self_test() {
        Some(paste::deliver(&text, target)?)
    } else {
        None
    };
    let paste_ms = paste_started.elapsed().as_secs_f64() * 1000.0;

    let entry = if has_words {
        let saved = state.history.as_ref().map_err(Clone::clone).and_then(|history| {
            history.add(&NewEntry {
                text: &text,
                app_name: &target.app_name(),
                word_count: word_count as i64,
                speech_seconds: recognized.speech_seconds,
                pasted: outcome == Some(PasteOutcome::Pasted),
            })
        });
        match saved {
            Ok(entry) => {
                let _ = app.emit("history-added", &entry);
                Some(entry)
            }
            Err(message) => {
                let _ = app.emit("history-problem", &message);
                None
            }
        }
    } else {
        None
    };

    let timings = Timings {
        shortcut_to_recording_ms: session.shortcut_ms,
        mic_ready_ms: session.mic_ready_ms.lock().ok().and_then(|ready| *ready),
        recorded_seconds,
        speech_seconds: recognized.speech_seconds,
        chunk_count: recognized.chunk_count,
        cleanup_ms: recognized.cleanup_ms,
        recognition_ms: recognized.recognition_ms,
        tidy_ms,
        stop_to_text_ms,
        paste_ms,
        total_ms: stopped.elapsed().as_secs_f64() * 1000.0,
        outcome: match &outcome {
            Some(PasteOutcome::Pasted) => "pasted",
            Some(PasteOutcome::CopiedInstead { .. }) => "copied",
            Some(PasteOutcome::StayedInMimi) => "stayed in Mimi",
            None => "nothing to paste",
        }
        .to_string(),
    };
    record_timings(&state.timings_file, &timings);

    Ok(Dictation {
        text,
        heard_speech,
        word_count,
        outcome,
        entry,
        timings,
    })
}

/// Applies the cleanup rules, if they're on, then inserts snippets. Parakeet detects the language,
/// so none is passed for it. A snippet problem never costs the person their dictation.
fn tidied(text: &str, state: &AppState) -> Result<String, String> {
    let cleaned = {
        let settings = state.settings.lock().map_err(|_| "Mimi hit an internal error.".to_string())?;
        if settings.cleanup {
            let language = if settings.model == ModelChoice::Parakeet { None } else { settings.language.as_deref() };
            tidy::tidy(text, language)
        } else {
            text.to_string()
        }
    };
    let saved = state.history.as_ref().ok().and_then(|history| history.snippets().ok()).unwrap_or_default();
    Ok(snippets::expand(&cleaned, &saved))
}

/// Whether to play the chimes and lower other apps. The hidden self test never does either,
/// because it runs on the owner's PC.
fn listening_sounds(state: &AppState) -> (bool, bool) {
    if is_self_test() {
        return (false, false);
    }
    state.settings.lock().map_or((true, true), |settings| (settings.chimes, settings.quiet_other_apps))
}

/// Windows lowers each other app, so its chime can play over the fade. A Mac lowers everything,
/// chimes included, so the volume only drops once the chime has played.
fn start_sounds(state: &AppState, chimes: bool, quiet_other_apps: bool) {
    if cfg!(target_os = "macos") {
        if chimes {
            sounds::play(Chime::Start);
        }
        if quiet_other_apps {
            state.quieter.lower_after(if chimes { sounds::CHIME_LENGTH } else { Duration::ZERO });
        }
        return;
    }
    // Other apps fade out while the chime plays and the microphone opens.
    if quiet_other_apps {
        state.quieter.lower();
    }
    if chimes {
        sounds::play(Chime::Start);
    }
}

/// On a Mac the chime waits for the volume to come back, on its own thread so text isn't delayed.
fn stop_sounds(state: &AppState, chimes: bool, quiet_other_apps: bool) {
    if cfg!(target_os = "macos") {
        let quieter = state.quieter.clone();
        thread::spawn(move || {
            if quiet_other_apps {
                quieter.restore_and_wait();
            }
            if chimes {
                sounds::play(Chime::Stop);
            }
        });
        return;
    }
    if chimes {
        sounds::play(Chime::Stop);
    }
    if quiet_other_apps {
        state.quieter.restore();
    }
}

fn go_idle(app: &AppHandle) {
    announce(app, Stage::Idle);
    if !is_self_test() {
        indicator::hide(app);
    }
}

fn notify_then_idle(app: &AppHandle, message: &str) {
    if is_self_test() {
        announce(app, Stage::Idle);
        return;
    }
    indicator::show(app);
    announce(app, Stage::Notice { message: message.to_string() });
    let app = app.clone();
    thread::spawn(move || {
        thread::sleep(NOTICE_TIME);
        let state = app.state::<AppState>();
        let quiet = state.session.lock().is_ok_and(|session| session.is_none());
        if quiet && !state.busy.load(Ordering::Acquire) {
            go_idle(&app);
        }
    });
}

/// Saves timing numbers for checking speed. Never includes words or audio.
fn record_timings(file: &Option<PathBuf>, timings: &Timings) {
    let Some(path) = file else { return };
    if let Some(folder) = path.parent() {
        let _ = std::fs::create_dir_all(folder);
    }
    if let (Ok(mut output), Ok(line)) = (
        std::fs::OpenOptions::new().create(true).append(true).open(path),
        serde_json::to_string(timings),
    ) {
        let _ = writeln!(output, "{line}");
    }
}

#[tauri::command]
fn engine_status(state: State<AppState>) -> EngineStatus {
    state.engine.status()
}

fn engine_options(settings: &Settings) -> EngineOptions {
    EngineOptions {
        model: settings.model,
        language: settings.language.clone(),
        punctuation_hint: true,
        cpu_only: settings.whisper_cpu_only,
    }
}

#[tauri::command]
fn get_settings(state: State<AppState>) -> Result<Settings, String> {
    Ok(state.settings.lock().map_err(|_| "Mimi hit an internal error.".to_string())?.clone())
}

#[tauri::command]
fn list_models() -> Vec<ModelInfo> {
    engine::list_models()
}

/// Saves the choice and applies it. A different model loads in the background.
#[tauri::command]
fn change_settings(model: ModelChoice, language: Option<String>, state: State<AppState>) -> Result<Settings, String> {
    let mut settings = state.settings.lock().map_err(|_| "Mimi hit an internal error.".to_string())?;
    settings.model = model;
    settings.language = language.filter(|code| !code.is_empty());
    settings::save(&state.data_folder, &settings)?;
    state.engine.configure(engine_options(&settings))?;
    Ok(settings.clone())
}

#[tauri::command]
fn set_cleanup(enabled: bool, state: State<AppState>) -> Result<Settings, String> {
    let mut settings = state.settings.lock().map_err(|_| "Mimi hit an internal error.".to_string())?;
    settings.cleanup = enabled;
    settings::save(&state.data_folder, &settings)?;
    Ok(settings.clone())
}

#[tauri::command]
fn set_start_with_windows(enabled: bool, state: State<AppState>) -> Result<Settings, String> {
    let mut settings = state.settings.lock().map_err(|_| "Mimi hit an internal error.".to_string())?;
    autostart::apply(enabled)?;
    settings.start_with_windows = enabled;
    settings::save(&state.data_folder, &settings)?;
    Ok(settings.clone())
}

#[tauri::command]
fn set_chimes(enabled: bool, state: State<AppState>) -> Result<Settings, String> {
    let mut settings = state.settings.lock().map_err(|_| "Mimi hit an internal error.".to_string())?;
    settings.chimes = enabled;
    settings::save(&state.data_folder, &settings)?;
    Ok(settings.clone())
}

#[tauri::command]
fn set_quiet_other_apps(enabled: bool, state: State<AppState>) -> Result<Settings, String> {
    let mut settings = state.settings.lock().map_err(|_| "Mimi hit an internal error.".to_string())?;
    settings.quiet_other_apps = enabled;
    settings::save(&state.data_folder, &settings)?;
    Ok(settings.clone())
}

/// Waits for the new keys, checks them and saves them. `Ok(None)` means it was cancelled.
#[tauri::command(async)]
fn change_shortcut(which: String, app: AppHandle) -> Result<Option<Settings>, String> {
    let state = app.state::<AppState>();
    let changing_hold = which == "hold";
    let Some(pressed) = hotkey::capture(Duration::from_secs(20))? else {
        return Ok(None);
    };
    let mut settings = state.settings.lock().map_err(|_| "Mimi hit an internal error.".to_string())?;
    if changing_hold {
        settings.hold_shortcut = hotkey::shortcut_from(&pressed, &settings.tap_shortcut, "tap to start and stop")?;
    } else {
        settings.tap_shortcut = hotkey::shortcut_from(&pressed, &settings.hold_shortcut, "hold to talk")?;
    }
    settings::save(&state.data_folder, &settings)?;
    hotkey::configure(settings.hold_shortcut.clone(), settings.tap_shortcut.clone());
    update_tray_tooltip(&app, &settings.hold_shortcut);
    Ok(Some(settings.clone()))
}

/// Lets the screen add a line to the diagnostics log, such as how a button was pressed.
#[tauri::command]
fn note_from_screen(text: String) {
    diag::note(format!("screen: {}", text.chars().take(200).collect::<String>()));
}

/// Mimi's window saw one of the shortcuts: "hold-down", "hold-up", "tap-down" or "tap-up".
#[tauri::command]
fn shortcut_from_screen(action: String) {
    let hold = action.starts_with("hold");
    let down = action.ends_with("down");
    hotkey::from_screen(hold, down);
}

/// Mimi's window saw the new keys while a shortcut was being changed.
#[tauri::command]
fn shortcut_captured_from_screen(ctrl: bool, alt: bool, shift: bool, win: bool, key: u32) {
    hotkey::captured_from_screen(hotkey::Pressed { modifiers: hotkey::Modifiers { ctrl, alt, shift, win }, key });
}

#[tauri::command]
fn cancel_shortcut_change() {
    hotkey::cancel_capture();
}

#[tauri::command]
fn reset_shortcuts(app: AppHandle) -> Result<Settings, String> {
    let state = app.state::<AppState>();
    let mut settings = state.settings.lock().map_err(|_| "Mimi hit an internal error.".to_string())?;
    settings.hold_shortcut = Shortcut::hold_default();
    settings.tap_shortcut = Shortcut::tap_default();
    settings::save(&state.data_folder, &settings)?;
    hotkey::configure(settings.hold_shortcut.clone(), settings.tap_shortcut.clone());
    update_tray_tooltip(&app, &settings.hold_shortcut);
    Ok(settings.clone())
}

fn tray_tooltip(hold: &Shortcut) -> String {
    format!("Mimi: hold {} to dictate", hold.label())
}

fn update_tray_tooltip(app: &AppHandle, hold: &Shortcut) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_tooltip(Some(tray_tooltip(hold)));
    }
}

#[tauri::command]
fn set_appearance(appearance: settings::Appearance, app: AppHandle) -> Result<Settings, String> {
    let state = app.state::<AppState>();
    let mut settings = state.settings.lock().map_err(|_| "Mimi hit an internal error.".to_string())?;
    settings.appearance = appearance;
    settings::save(&state.data_folder, &settings)?;
    // The recording indicator is its own window, so it hears about the change here.
    let _ = app.emit("appearance-changed", appearance);
    Ok(settings.clone())
}

/// Lets the recording indicator start in the right colors.
#[tauri::command]
fn get_appearance(state: State<AppState>) -> settings::Appearance {
    state.settings.lock().map(|settings| settings.appearance).unwrap_or_default()
}

#[tauri::command]
fn set_animate_mimi(enabled: bool, state: State<AppState>) -> Result<Settings, String> {
    let mut settings = state.settings.lock().map_err(|_| "Mimi hit an internal error.".to_string())?;
    settings.animate_mimi = enabled;
    settings::save(&state.data_folder, &settings)?;
    Ok(settings.clone())
}

#[tauri::command(async)]
fn list_snippets(state: State<AppState>) -> Result<Vec<Snippet>, String> {
    state.history.as_ref().map_err(Clone::clone)?.snippets()
}

#[tauri::command(async)]
fn save_snippet(id: Option<i64>, phrase: String, text: String, state: State<AppState>) -> Result<Snippet, String> {
    state.history.as_ref().map_err(Clone::clone)?.save_snippet(id, &phrase, &text)
}

#[tauri::command(async)]
fn delete_snippet(id: i64, state: State<AppState>) -> Result<bool, String> {
    state.history.as_ref().map_err(Clone::clone)?.delete_snippet(id)
}

#[tauri::command]
fn quit_app(app: AppHandle) {
    app.exit(0);
}

#[tauri::command(async)]
fn list_microphones() -> Result<Vec<MicInfo>, String> {
    audio::list_microphones()
}

#[tauri::command]
fn set_microphone(device: Option<String>, state: State<AppState>) -> Result<(), String> {
    *state.chosen_microphone.lock().map_err(|_| "Mimi hit an internal error.".to_string())? =
        device.filter(|name| !name.is_empty());
    Ok(())
}

#[tauri::command(async)]
fn start_recording(device: Option<String>, app: AppHandle) -> Result<(), String> {
    begin(&app, device, None)?;
    hotkey::set_recording(true);
    Ok(())
}

#[tauri::command(async)]
fn stop_recording(app: AppHandle) -> Result<Dictation, String> {
    finish(&app)
}

#[tauri::command(async)]
fn history_recent(before_id: Option<i64>, state: State<AppState>) -> Result<Vec<HistoryEntry>, String> {
    state.history.as_ref().map_err(Clone::clone)?.recent(HISTORY_PAGE, before_id)
}

#[tauri::command(async)]
fn history_search(query: String, state: State<AppState>) -> Result<Vec<HistoryEntry>, String> {
    state.history.as_ref().map_err(Clone::clone)?.search(&query, HISTORY_SEARCH_LIMIT)
}

#[tauri::command(async)]
fn history_delete(id: i64, state: State<AppState>) -> Result<bool, String> {
    state.history.as_ref().map_err(Clone::clone)?.delete(id)
}

#[tauri::command(async)]
fn history_count(state: State<AppState>) -> Result<i64, String> {
    state.history.as_ref().map_err(Clone::clone)?.count()
}

/// Deletes every dictation. Snippets stay.
#[tauri::command(async)]
fn history_clear(state: State<AppState>) -> Result<usize, String> {
    state.history.as_ref().map_err(Clone::clone)?.clear()
}

#[tauri::command(async)]
fn copy_text(text: String) -> Result<(), String> {
    paste::copy_to_clipboard(&text)
}

/// The hidden end-to-end test, switched on only by environment variables.
#[derive(Debug, Clone, Serialize)]
struct SelfTestPlan {
    clips: Vec<String>,
}

#[tauri::command]
fn self_test_plan() -> Option<SelfTestPlan> {
    std::env::var("MIMI_SELF_TEST_OUT").ok()?;
    let clips = std::env::var("MIMI_SELF_TEST_CLIPS").ok()?;
    Some(SelfTestPlan {
        clips: clips.split(';').map(str::to_string).collect(),
    })
}

#[tauri::command]
fn self_test_finish(report: serde_json::Value, app: AppHandle) {
    if let Ok(path) = std::env::var("MIMI_SELF_TEST_OUT") {
        let body = serde_json::to_string_pretty(&report).unwrap_or_default();
        let _ = std::fs::write(path, body);
    }
    app.exit(0);
}

/// The window starts hidden and appears once its first screen is ready, so it never flashes white.
#[tauri::command]
fn window_ready(app: AppHandle) {
    if SHOW_WHEN_READY.load(Ordering::Acquire) {
        show_main_window(&app);
    }
}

fn show_main_window(app: &AppHandle) {
    if is_self_test() {
        return;
    }
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// Listens for the shortcuts on their own thread, so a slow microphone never delays the keyboard.
fn start_shortcut(app: AppHandle) -> Result<(), String> {
    let (sender, inbox) = channel::<HotkeyEvent>();
    let (hold, tap) = {
        let state = app.state::<AppState>();
        let settings = state.settings.lock().map_err(|_| "Mimi hit an internal error.".to_string())?;
        (settings.hold_shortcut.clone(), settings.tap_shortcut.clone())
    };
    hotkey::install(sender, hold, tap)?;
    thread::spawn(move || {
        for event in inbox {
            match event {
                HotkeyEvent::Start => {
                    let pressed = Instant::now();
                    if begin(&app, None, Some(pressed)).is_err() {
                        hotkey::set_recording(false);
                    }
                }
                HotkeyEvent::Stop => {
                    let worker = app.clone();
                    thread::spawn(move || {
                        let _ = finish(&worker);
                    });
                }
                HotkeyEvent::Captured(_) => {}
            }
        }
    });
    Ok(())
}

fn start_tray(app: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open Mimi", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit Mimi", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &quit])?;
    let hold = app.state::<AppState>().settings.lock().map(|settings| settings.hold_shortcut.clone()).unwrap_or_else(|_| Shortcut::hold_default());
    let mut tray = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip(tray_tooltip(&hold))
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => show_main_window(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main_window(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        tray = tray.icon(icon.clone());
    }
    tray.build(app)?;
    Ok(())
}

/// Where Mimi keeps her settings and dictations.
fn default_data_folder() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    let folder = std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Library/Application Support/Mimi"));
    #[cfg(not(target_os = "macos"))]
    let folder = std::env::var_os("LOCALAPPDATA").map(|local| PathBuf::from(local).join("Mimi"));
    folder
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    engine::prepare_graphics_environment();
    let mut builder = tauri::Builder::default();
    // The hidden self test must never take focus from whatever the owner is doing.
    #[cfg(target_os = "macos")]
    if is_self_test() {
        builder = builder.activate_ignoring_other_apps(false);
    }
    // A second copy would listen for the same shortcut and paste everything twice.
    // The hidden self test is allowed to run beside a copy that's already open.
    if !is_self_test() {
        builder = builder.plugin(tauri_plugin_single_instance::init(|app, _, _| show_main_window(app)));
    }
    builder
        .setup(|app| {
            #[cfg(target_os = "macos")]
            if is_self_test() {
                app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            }
            // Tests use their own folder, so they never touch real settings or dictations.
            let data_folder = std::env::var_os("MIMI_DATA_DIR")
                .map(PathBuf::from)
                .or_else(default_data_folder)
                .unwrap_or_else(std::env::temp_dir);
            let timings_file = Some(data_folder.join("timings.jsonl"));
            if !is_self_test() {
                diag::start(data_folder.join("diagnostics.log"));
            }
            let volume_memory = data_folder.join("other-apps-volume.json");
            let history = if is_self_test() {
                History::open_in_memory()
            } else {
                History::open(&data_folder.join("history.db"))
            };

            let mut saved = settings::load(&data_folder);
            if autostart::started_hidden() {
                SHOW_WHEN_READY.store(false, Ordering::Release);
            }
            // A crash while first using the graphics card shouldn't happen twice.
            if engine::graphics_card_crashed_last_time(&data_folder) {
                saved.whisper_cpu_only = true;
                let _ = settings::save(&data_folder, &saved);
            }

            app.manage(AppState {
                microphone: Microphone::new(),
                engine: Engine::start(engine_options(&saved), data_folder.clone()),
                settings: Mutex::new(saved),
                data_folder,
                history,
                session: Mutex::new(None),
                busy: AtomicBool::new(false),
                chosen_microphone: Mutex::new(None),
                timings_file,
                // Also restores any app left quiet if Mimi closed unexpectedly while listening.
                quieter: quiet::Quieter::start(volume_memory),
            });

            indicator::prepare(app.handle());
            if !is_self_test() {
                let start_with_windows = app.state::<AppState>().settings.lock().is_ok_and(|saved| saved.start_with_windows);
                if let Err(message) = autostart::apply(start_with_windows) {
                    eprintln!("{message}");
                }
                start_tray(app.handle())?;
                if let Err(message) = start_shortcut(app.handle().clone()) {
                    let _ = app.emit("shortcut-problem", message);
                }
                // Safety net in case the screen fails to report that it's ready.
                let handle = app.handle().clone();
                thread::spawn(move || {
                    thread::sleep(Duration::from_secs(4));
                    if SHOW_WHEN_READY.load(Ordering::Acquire)
                        && let Some(window) = handle.get_webview_window("main") {
                        let _ = window.show();
                    }
                });
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing the window keeps Mimi running in the tray, so the shortcut keeps working.
            if window.label() == "main"
                && let WindowEvent::CloseRequested { api, .. } = event
            {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            engine_status,
            get_settings,
            list_models,
            change_settings,
            set_cleanup,
            set_animate_mimi,
            set_start_with_windows,
            set_chimes,
            set_quiet_other_apps,
            set_appearance,
            get_appearance,
            change_shortcut,
            cancel_shortcut_change,
            note_from_screen,
            shortcut_from_screen,
            shortcut_captured_from_screen,
            reset_shortcuts,
            list_snippets,
            save_snippet,
            delete_snippet,
            quit_app,
            list_microphones,
            set_microphone,
            start_recording,
            stop_recording,
            history_recent,
            history_search,
            history_delete,
            history_count,
            history_clear,
            copy_text,
            self_test_plan,
            self_test_finish,
            window_ready
        ])
        .build(tauri::generate_context!())
        .expect("error while starting Mimi")
        .run(|app, event| {
            if let RunEvent::Exit = event {
                app.state::<AppState>().quieter.restore_now();
                engine::clear_graphics_card_check(&app.state::<AppState>().data_folder);
                // whisper.cpp's Metal cleanup aborts if a model is still loaded while the program
                // closes, which macOS would report as a crash. Everything is saved by now.
                #[cfg(target_os = "macos")]
                unsafe {
                    _exit(0)
                };
            }
        });
}

#[cfg(target_os = "macos")]
unsafe extern "C" {
    fn _exit(code: i32) -> !;
}
