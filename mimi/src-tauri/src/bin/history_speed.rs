//! Measures the history with 10,000 dictations in a temporary database file.
//!
//! Usage: history_speed <output.json>

use mimi_lib::history::{History, NewEntry};
use serde::Serialize;
use std::time::Instant;

const ENTRIES: usize = 10_000;
const REPEATS: usize = 25;
const SEARCH_LIMIT: usize = 200;
const PAGE: usize = 100;

const SPANISH: &[&str] = &[
    "hola", "reunión", "mañana", "cliente", "presupuesto", "proyecto", "gracias", "mensaje", "enviar", "video", "edición", "equipo",
    "semana", "correo", "archivo", "revisar", "dirección", "teléfono", "tiempo", "trabajo", "entrega", "viernes", "lunes", "campaña",
    "creo", "que", "podemos", "vernos", "para", "con", "el", "la", "de", "por", "favor", "después", "también", "nuevo",
];
const ENGLISH: &[&str] = &[
    "hello", "meeting", "tomorrow", "client", "budget", "project", "thanks", "message", "send", "video", "editing", "team", "week",
    "email", "file", "review", "address", "phone", "time", "work", "delivery", "friday", "monday", "campaign", "think", "that", "we",
    "can", "meet", "for", "with", "the", "of", "please", "after", "also", "new",
];
const JAPANESE: &[&str] = &["今日は予約がたくさんあって疲れました。", "明日の会議は十時からです。", "荷物を送ってください。", "ありがとうございます。", "動画の編集が終わりました。"];

/// A small repeatable random number generator, so every run tests the same data.
struct Dice(u64);

impl Dice {
    fn next(&mut self, below: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % below as u64) as usize
    }
}

fn dictation(dice: &mut Dice) -> String {
    let kind = dice.next(10);
    if kind == 0 {
        return (0..1 + dice.next(3)).map(|_| JAPANESE[dice.next(JAPANESE.len())]).collect();
    }
    let words = if kind < 6 { SPANISH } else { ENGLISH };
    let length = 8 + dice.next(50);
    let mut text: String = (0..length).map(|_| words[dice.next(words.len())]).collect::<Vec<_>>().join(" ");
    text.push('.');
    text
}

#[derive(Serialize)]
struct Timing {
    action: String,
    matches: usize,
    typical_ms: f64,
    slowest_ms: f64,
}

#[derive(Serialize)]
struct Report {
    entries: i64,
    file_mb: f64,
    fill_seconds: f64,
    timings: Vec<Timing>,
}

fn measure(action: &str, mut work: impl FnMut() -> usize) -> Timing {
    let mut times = Vec::with_capacity(REPEATS);
    let mut matches = 0;
    for _ in 0..REPEATS {
        let started = Instant::now();
        matches = work();
        times.push(started.elapsed().as_secs_f64() * 1000.0);
    }
    times.sort_by(f64::total_cmp);
    Timing { action: action.to_string(), matches, typical_ms: times[times.len() / 2], slowest_ms: times[times.len() - 1] }
}

fn main() {
    let output = std::env::args().nth(1).expect("usage: history_speed <output.json>");
    let folder = std::env::temp_dir().join(format!("mimi-history-speed-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&folder);
    let path = folder.join("history.db");
    let history = History::open(&path).unwrap();

    let mut dice = Dice(0x4D49_4D49_2026);
    let started = Instant::now();
    for _ in 0..ENTRIES {
        let text = dictation(&mut dice);
        history
            .add(&NewEntry { text: &text, app_name: "Chrome", word_count: 20, speech_seconds: 10.0, pasted: true })
            .unwrap();
    }
    let fill_seconds = started.elapsed().as_secs_f64();

    let mut timings = Vec::new();
    for query in ["reunion", "MAÑANA", "presupuesto cliente", "pres", "meeting budget", "予約", "編集が", "nothing like this"] {
        timings.push(measure(&format!("Search: {query}"), || history.search(query, SEARCH_LIMIT).unwrap().len()));
    }
    timings.push(measure("Open: newest 100", || history.recent(PAGE, None).unwrap().len()));
    let middle = history.recent(1, None).unwrap()[0].id - (ENTRIES as i64 / 2);
    timings.push(measure("Scroll: 100 older, halfway back", || history.recent(PAGE, Some(middle)).unwrap().len()));
    timings.push(measure("Save a dictation", || {
        history.add(&NewEntry { text: "una prueba de velocidad", app_name: "Chrome", word_count: 4, speech_seconds: 2.0, pasted: true }).unwrap();
        1
    }));
    let doomed = history.recent(REPEATS, None).unwrap();
    let mut next = doomed.iter();
    timings.push(measure("Delete a dictation", || {
        history.delete(next.next().unwrap().id).unwrap();
        1
    }));

    let report = Report {
        entries: history.count().unwrap(),
        file_mb: std::fs::metadata(&path).map_or(0.0, |file| file.len() as f64 / 1_048_576.0),
        fill_seconds,
        timings,
    };
    std::fs::write(&output, serde_json::to_string_pretty(&report).unwrap()).unwrap();
    println!("| Action | Matches | Typical | Slowest |\n|---|---|---|---|");
    for timing in &report.timings {
        println!("| {} | {} | {:.2} ms | {:.2} ms |", timing.action, timing.matches, timing.typical_ms, timing.slowest_ms);
    }
    println!("{} dictations, {:.1} MB, filled in {:.1} s", report.entries, report.file_mb, report.fill_seconds);
    drop(history);
    let _ = std::fs::remove_dir_all(&folder);
}
