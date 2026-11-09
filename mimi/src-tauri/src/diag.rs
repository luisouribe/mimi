//! A short log for finding problems: when shortcut keys arrive, what Mimi decides, and how each
//! recording ends. It never contains words, only times, key names and numbers. It lives in
//! `diagnostics.log` in Mimi's data folder and is kept under 256 KB.

use std::io::Write;
use std::path::PathBuf;
use std::sync::mpsc::{Sender, channel};
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

const LIMIT: u64 = 256 * 1024;

static WRITER: OnceLock<Mutex<Sender<String>>> = OnceLock::new();
static STARTED: OnceLock<Instant> = OnceLock::new();

/// Starts the writer. Lines are written on their own thread, so the keyboard hook never waits for the disk.
pub fn start(path: PathBuf) {
    let (sender, inbox) = channel::<String>();
    if WRITER.set(Mutex::new(sender)).is_err() {
        return;
    }
    STARTED.get_or_init(Instant::now);
    std::thread::Builder::new()
        .name("mimi-diagnostics".into())
        .spawn(move || {
            if std::fs::metadata(&path).is_ok_and(|meta| meta.len() > LIMIT) {
                let _ = std::fs::remove_file(&path);
            }
            let stamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |since| since.as_secs());
            let mut file = std::fs::OpenOptions::new().create(true).append(true).open(&path).ok();
            if let Some(file) = file.as_mut() {
                let _ = writeln!(file, "--- Mimi started, unix time {stamp}");
            }
            for line in inbox {
                if let Some(file) = file.as_mut() {
                    let _ = writeln!(file, "{line}");
                    let _ = file.flush();
                }
            }
        })
        .ok();
}

/// Adds one line, with the milliseconds since Mimi started.
pub fn note(text: impl AsRef<str>) {
    let Some(writer) = WRITER.get() else { return };
    let ms = STARTED.get().map_or(0, |started| started.elapsed().as_millis());
    if let Ok(sender) = writer.lock() {
        let _ = sender.send(format!("{ms:>9} {}", text.as_ref()));
    }
}
