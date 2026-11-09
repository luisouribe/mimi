//! Tests the cleanup rules without any audio.
//!
//! Usage: tidy_check clips <report.json>...   Lists every saved recognition the rules would change.
//!        tidy_check lines <file>             Prints each line of a text file before and after.

use mimi_lib::snippets::{Snippet, expand};
use mimi_lib::tidy::tidy;
use serde::Deserialize;

#[derive(Deserialize)]
struct Clip {
    lang: String,
    file: String,
    text: String,
    reference: String,
}

#[derive(Deserialize)]
struct Report {
    clips: Vec<Clip>,
}

fn normal_spaces(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn check_clips(paths: &[String]) {
    let (mut total, mut changed) = (0, 0);
    for path in paths {
        let body = std::fs::read_to_string(path).unwrap();
        let report: Report = serde_json::from_str(&body).unwrap();
        for clip in &report.clips {
            for (kind, text) in [("recognized", &clip.text), ("reference", &clip.reference)] {
                for language in [None, Some(clip.lang.as_str())] {
                    total += 1;
                    let result = tidy(text, language);
                    if result != normal_spaces(text) {
                        changed += 1;
                        println!("CHANGED {path} {} {kind} [{language:?}]\n  before: {text}\n  after:  {result}", clip.file);
                    }
                }
            }
        }
    }
    println!("{total} texts checked, {changed} changed");
}

/// Counts how often normal speech would set off snippets people are likely to save.
fn check_snippets(paths: &[String]) {
    let saved: Vec<Snippet> = [
        "mi dirección", "my address", "mi correo", "my email", "mi teléfono", "my phone number", "mi enlace", "my link",
        "mi nombre", "my name", "mi bio", "my bio", "respuesta estándar", "standard reply", "mis datos", "my details",
    ]
    .iter()
    .enumerate()
    .map(|(index, phrase)| Snippet { id: index as i64, phrase: phrase.to_string(), text: "<<SNIPPET>>".to_string() })
    .collect();
    let (mut total, mut set_off) = (0, 0);
    for path in paths {
        let body = std::fs::read_to_string(path).unwrap();
        let report: Report = serde_json::from_str(&body).unwrap();
        for clip in &report.clips {
            for text in [&clip.text, &clip.reference] {
                total += 1;
                if expand(text, &saved).contains("<<SNIPPET>>") {
                    set_off += 1;
                    println!("SET OFF {}: {text}", clip.file);
                }
            }
        }
    }
    println!("{total} texts checked against {} phrases, {set_off} set off a snippet", saved.len());
}

fn check_lines(path: &str) {
    for line in std::fs::read_to_string(path).unwrap().lines().filter(|line| !line.trim().is_empty()) {
        println!("{line}\n  => {:?}", tidy(line, None));
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("clips") => check_clips(&args[1..]),
        Some("snippets") => check_snippets(&args[1..]),
        Some("lines") => check_lines(&args[1]),
        _ => panic!("usage: tidy_check clips <report.json>... | lines <file>"),
    }
}
