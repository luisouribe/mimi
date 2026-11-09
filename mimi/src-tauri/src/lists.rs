//! Spoken lists. Counting out loud becomes a numbered list, and "viñeta" or "bullet" before each
//! item becomes a bulleted one:
//!
//! "Necesito, primero, comprar pan, segundo, llamar a Juan." becomes
//! "Necesito:\n1. Comprar pan.\n2. Llamar a Juan."
//!
//! Counting only counts when it reaches at least two in order, starting at one, and each item has
//! words. With pauses written ("primero, ... segundo, ...") two are enough. People also count with
//! no pauses at all ("primero ver si jala y dos ver si corre y tres ..."), so a run of three or more
//! counts too, and so do two when the second number is cut off by a pause. "Primero voy a comer"
//! and "compré dos panes y tres leches" stay sentences.

use crate::tidy::{Piece, Word, capitalized, fold, is_hard_stop, LINE};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Family {
    SpanishOrdinal,
    EnglishOrdinal,
    SpanishNumber,
    EnglishNumber,
    Digit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Number(u8, Family),
    /// "Por último" or "finally": the next number, after at least two.
    Last,
    Bullet,
}

/// Phrases that count, written in lowercase with no accents.
const COUNTING: &[(&[&str], u8, Family)] = &[
    (&["primero", "que", "nada"], 1, Family::SpanishOrdinal),
    (&["en", "primer", "lugar"], 1, Family::SpanishOrdinal),
    (&["en", "segundo", "lugar"], 2, Family::SpanishOrdinal),
    (&["en", "tercer", "lugar"], 3, Family::SpanishOrdinal),
    (&["en", "cuarto", "lugar"], 4, Family::SpanishOrdinal),
    (&["first", "of", "all"], 1, Family::EnglishOrdinal),
    (&["primero"], 1, Family::SpanishOrdinal),
    (&["primera"], 1, Family::SpanishOrdinal),
    (&["segundo"], 2, Family::SpanishOrdinal),
    (&["segunda"], 2, Family::SpanishOrdinal),
    (&["tercero"], 3, Family::SpanishOrdinal),
    (&["tercera"], 3, Family::SpanishOrdinal),
    (&["cuarto"], 4, Family::SpanishOrdinal),
    (&["cuarta"], 4, Family::SpanishOrdinal),
    (&["quinto"], 5, Family::SpanishOrdinal),
    (&["quinta"], 5, Family::SpanishOrdinal),
    (&["sexto"], 6, Family::SpanishOrdinal),
    (&["sexta"], 6, Family::SpanishOrdinal),
    (&["septimo"], 7, Family::SpanishOrdinal),
    (&["septima"], 7, Family::SpanishOrdinal),
    (&["octavo"], 8, Family::SpanishOrdinal),
    (&["octava"], 8, Family::SpanishOrdinal),
    (&["noveno"], 9, Family::SpanishOrdinal),
    (&["novena"], 9, Family::SpanishOrdinal),
    (&["decimo"], 10, Family::SpanishOrdinal),
    (&["decima"], 10, Family::SpanishOrdinal),
    (&["first"], 1, Family::EnglishOrdinal),
    (&["firstly"], 1, Family::EnglishOrdinal),
    (&["second"], 2, Family::EnglishOrdinal),
    (&["secondly"], 2, Family::EnglishOrdinal),
    (&["third"], 3, Family::EnglishOrdinal),
    (&["thirdly"], 3, Family::EnglishOrdinal),
    (&["fourth"], 4, Family::EnglishOrdinal),
    (&["fifth"], 5, Family::EnglishOrdinal),
    (&["sixth"], 6, Family::EnglishOrdinal),
    (&["seventh"], 7, Family::EnglishOrdinal),
    (&["eighth"], 8, Family::EnglishOrdinal),
    (&["ninth"], 9, Family::EnglishOrdinal),
    (&["tenth"], 10, Family::EnglishOrdinal),
    (&["uno"], 1, Family::SpanishNumber),
    (&["dos"], 2, Family::SpanishNumber),
    (&["tres"], 3, Family::SpanishNumber),
    (&["cuatro"], 4, Family::SpanishNumber),
    (&["cinco"], 5, Family::SpanishNumber),
    (&["seis"], 6, Family::SpanishNumber),
    (&["siete"], 7, Family::SpanishNumber),
    (&["ocho"], 8, Family::SpanishNumber),
    (&["nueve"], 9, Family::SpanishNumber),
    (&["diez"], 10, Family::SpanishNumber),
    (&["one"], 1, Family::EnglishNumber),
    (&["two"], 2, Family::EnglishNumber),
    (&["three"], 3, Family::EnglishNumber),
    (&["four"], 4, Family::EnglishNumber),
    (&["five"], 5, Family::EnglishNumber),
    (&["six"], 6, Family::EnglishNumber),
    (&["seven"], 7, Family::EnglishNumber),
    (&["eight"], 8, Family::EnglishNumber),
    (&["nine"], 9, Family::EnglishNumber),
    (&["ten"], 10, Family::EnglishNumber),
];

/// Ways to say the last item, after at least two numbered ones.
const LAST: &[&[&str]] = &[&["por", "ultimo"], &["finalmente"], &["finally"], &["lastly"]];

/// Words that introduce a bullet.
const BULLETS: &[&[&str]] = &[&["bullet", "point"], &["vineta"], &["bullet"]];

/// "Una viñeta" and "the bullet" are the nouns, not the command.
const ARTICLES: &[&str] = &[
    "la", "una", "esa", "esta", "cada", "otra", "su", "mi", "tu", "the", "a", "this", "that", "each", "every", "one", "my",
    "your", "his", "her", "its", "our", "their",
];

/// "And" before a number: "y tercero, mandar el video". It's dropped from the list.
const JOINING: &[&str] = &["y", "e", "and"];

/// The words that make up a list, ignoring line breaks from commands.
struct Marker {
    at: usize,
    length: usize,
    kind: Kind,
    /// The piece index of an "and" before it, which is dropped.
    joined: Option<usize>,
    /// Starts a clause after punctuation, and a plain number is cut off by a pause: the clearest kind.
    strict: bool,
    /// Starts a clause, at least after "y": "... las plantas y dos ...".
    clause: bool,
    /// A pause is written right after it.
    pause: bool,
}

/// Counting may mix ordinals and plain numbers of one language: "primero ..., dos ..., tres ...".
fn language(family: Family) -> u8 {
    match family {
        Family::SpanishOrdinal | Family::SpanishNumber => 0,
        Family::EnglishOrdinal | Family::EnglishNumber => 1,
        Family::Digit => 2,
    }
}

/// A digit counts along with words of either language: Parakeet can write "1 buscar ... Dos, el patio".
fn counts_together(a: Family, b: Family) -> bool {
    a == Family::Digit || b == Family::Digit || language(a) == language(b)
}

/// Words before a number that show it isn't counting a list: "el segundo", "en primer lugar",
/// "el 1 de mayo", "capítulo 2".
const NOT_BEFORE_COUNTING: &[&str] = &[
    "el", "la", "los", "las", "lo", "un", "una", "unos", "unas", "del", "al", "de", "en", "por", "a", "con", "para", "su",
    "sus", "mi", "mis", "tu", "tus", "este", "esta", "ese", "esa", "the", "an", "at", "on", "in", "of", "to", "by", "for",
    "my", "your", "his", "her", "its", "our", "their", "capitulo", "pagina", "numero", "version", "parte", "nivel",
    "episodio", "temporada", "paso", "chapter", "page", "number", "part", "level", "episode", "season", "step",
];

fn after_a_blocker(pieces: &[Piece], at: usize) -> bool {
    at.checked_sub(1)
        .and_then(|previous| word_at(pieces, previous))
        .is_some_and(|word| word.trail.is_empty() && NOT_BEFORE_COUNTING.contains(&key(word).as_str()))
}

/// Words ending in "s" that aren't plural nouns, so "2 los equipos" isn't a quantity.
const NOT_PLURAL: &[&str] = &[
    "los", "las", "unos", "unas", "mis", "tus", "sus", "nuestros", "nuestras", "esos", "esas", "estos", "estas", "mas",
    "pues", "this", "his", "is", "as", "us", "was", "has", "its",
];

/// True when the numbers are amounts of something: "1 kilo, 2 litros y 3 latas".
fn reads_as_quantities(pieces: &[Piece], run: &[(Marker, String)]) -> bool {
    let mut numbers = run
        .iter()
        .skip(1)
        .filter(|(marker, _)| {
            !marker.pause
                && matches!(marker.kind, Kind::Number(_, Family::SpanishNumber | Family::EnglishNumber | Family::Digit))
        })
        .peekable();
    numbers.peek().is_some()
        && numbers.all(|(marker, _)| {
            word_at(pieces, marker.at + marker.length).is_some_and(|word| {
                let word = key(word);
                word.len() > 2 && word.ends_with('s') && !NOT_PLURAL.contains(&word.as_str())
            })
        })
}

/// "y" right before, with no pause before it: "... las plantas y dos ...". Returns the "y".
fn joined_by_and(pieces: &[Piece], at: usize) -> Option<usize> {
    let previous = at.checked_sub(1)?;
    let word = word_at(pieces, previous)?;
    (word.trail.is_empty() && word.lead.is_empty() && JOINING.contains(&key(word).as_str())).then_some(previous)
}

fn word_at(pieces: &[Piece], at: usize) -> Option<&Word> {
    match pieces.get(at) {
        Some(Piece::Word(word)) => Some(word),
        _ => None,
    }
}

/// The key used to match a word: lowercase with no accents, and "viñeta" spelled as "vineta".
fn key(word: &Word) -> String {
    fold(&word.core).replace('ñ', "n")
}

fn phrase_at(pieces: &[Piece], at: usize, phrase: &[&str]) -> bool {
    phrase.iter().enumerate().all(|(offset, part)| {
        word_at(pieces, at + offset).is_some_and(|word| {
            // Only the last word may carry punctuation, and only the first may have any before it.
            key(word) == *part && (offset + 1 == phrase.len() || word.trail.is_empty()) && (offset == 0 || word.lead.is_empty())
        })
    })
}

fn ends_clause(trail: &str) -> bool {
    trail.contains([',', '.', ':', ';', '!', '?', '…'])
}

/// True when a phrase here starts a clause: at the very start, after a line break, after
/// punctuation, or after "and" that itself follows punctuation. Returns the "and", if any.
fn starts_clause(pieces: &[Piece], at: usize) -> Option<Option<usize>> {
    let Some(previous) = at.checked_sub(1) else { return Some(None) };
    match &pieces[previous] {
        Piece::Break(_) => Some(None),
        Piece::Word(word) if ends_clause(&word.trail) => Some(None),
        Piece::Word(word) if word.trail.is_empty() && JOINING.contains(&key(word).as_str()) => {
            let before_and = previous.checked_sub(1);
            let clean = before_and.is_none_or(|index| match &pieces[index] {
                Piece::Break(_) => true,
                Piece::Word(word) => ends_clause(&word.trail),
            });
            clean.then_some(Some(previous))
        }
        _ => None,
    }
}

fn after_an_article(pieces: &[Piece], at: usize) -> bool {
    at.checked_sub(1)
        .and_then(|previous| word_at(pieces, previous))
        .is_some_and(|word| word.trail.is_empty() && ARTICLES.contains(&key(word).as_str()))
}

/// True when something follows the phrase in the same stretch of text.
fn more_follows(pieces: &[Piece], after: usize) -> bool {
    word_at(pieces, after).is_some()
}

/// A bullet phrase here, even with no pause written around it.
fn bullet_at(pieces: &[Piece], at: usize) -> Option<usize> {
    if after_an_article(pieces, at) {
        return None;
    }
    BULLETS
        .iter()
        .find(|phrase| phrase_at(pieces, at, phrase) && more_follows(pieces, at + phrase.len()))
        .map(|phrase| phrase.len())
}

fn find_markers(pieces: &[Piece]) -> Vec<Marker> {
    // Parakeet sometimes writes no punctuation at all: "Shopping list Bullet Point Apples Bullet
    // Point Bread". Saying the bullet word twice or more is clear enough on its own.
    let bullets_said = (0..pieces.len()).filter(|&at| bullet_at(pieces, at).is_some()).count();
    let mut found = Vec::new();
    let mut at = 0;
    while at < pieces.len() {
        if word_at(pieces, at).is_none() {
            at += 1;
            continue;
        }
        let strict_clause = starts_clause(pieces, at);
        let and_before = if strict_clause.is_none() { joined_by_and(pieces, at) } else { None };
        let clause = strict_clause.is_some() || and_before.is_some();
        let joined = strict_clause.flatten().or(and_before);
        let marker_here = |length: usize, kind: Kind, strict: bool, pause: bool| Marker {
            at,
            length,
            kind,
            joined,
            strict,
            clause,
            pause,
        };
        let mut marker = None;
        for &(phrase, value, family) in COUNTING {
            if phrase_at(pieces, at, phrase) && more_follows(pieces, at + phrase.len()) {
                let last = word_at(pieces, at + phrase.len() - 1).expect("the phrase matched");
                let pause = ends_clause(&last.trail);
                // A plain number, such as "dos", is clearest when it's cut off by a pause.
                let plain = matches!(family, Family::SpanishNumber | Family::EnglishNumber);
                // In the middle of a sentence, as when the model writes no punctuation: "una lista
                // primero ver ... segundo ...". A plain number there needs its pause, or to be the first.
                if !clause && (after_a_blocker(pieces, at) || (plain && value != 1 && !pause)) {
                    continue;
                }
                let strict = strict_clause.is_some() && (pause || !plain);
                marker = Some(marker_here(phrase.len(), Kind::Number(value, family), strict, pause));
                break;
            }
        }
        // Parakeet often writes counting as digits with no punctuation: "hoy 1 lavar ... 2 hornear".
        if marker.is_none()
            && let Some(digit) = word_at(pieces, at).and_then(|word| word.core.parse::<u8>().ok())
            && (1..=20).contains(&digit)
            && word_at(pieces, at).is_some_and(|word| word.lead.is_empty())
            && (clause || !after_a_blocker(pieces, at))
            && word_at(pieces, at + 1).is_some_and(|next| !matches!(key(next).as_str(), "de" | "of"))
        {
            let pause = word_at(pieces, at).is_some_and(|word| ends_clause(&word.trail));
            marker = Some(marker_here(1, Kind::Number(digit, Family::Digit), strict_clause.is_some() && pause, pause));
        }
        if marker.is_none() && clause {
            marker = LAST
                .iter()
                .find(|phrase| phrase_at(pieces, at, phrase) && more_follows(pieces, at + phrase.len()))
                .map(|phrase| marker_here(phrase.len(), Kind::Last, strict_clause.is_some(), true));
        }
        if marker.is_none()
            && (strict_clause.is_some() || bullets_said >= 2)
            && let Some(length) = bullet_at(pieces, at)
        {
            marker = Some(marker_here(length, Kind::Bullet, true, true));
        }
        match marker {
            Some(marker) => {
                at += marker.length;
                found.push(marker);
            }
            None => at += 1,
        }
    }
    found
}

/// True when there are words between two markers: "uno, dos, tres, probando" is not a list.
fn has_words_between(pieces: &[Piece], before: &Marker, after: &Marker) -> bool {
    (before.at + before.length..after.at)
        .any(|index| Some(index) != after.joined && word_at(pieces, index).is_some())
}

/// Keeps the markers that really make a list: numbers counting 1, 2, 3... in one language, with
/// at least two, plus an optional "por último"; and every bullet.
fn real_lists(pieces: &[Piece], markers: Vec<Marker>) -> Vec<(Marker, String)> {
    let mut kept: Vec<(Marker, String)> = Vec::new();
    let mut run: Vec<(Marker, String)> = Vec::new();
    let mut family: Option<Family> = None;
    let flush = |run: &mut Vec<(Marker, String)>, kept: &mut Vec<(Marker, String)>| {
        let numbers = run.iter().filter(|(marker, _)| !matches!(marker.kind, Kind::Last)).count();
        let all_strict = run.iter().all(|(marker, _)| marker.strict);
        // With no pauses written, two could be chance ("primero comí y dos horas después ..."), so
        // two need the second cut off by a pause. Three in a row are clear on their own.
        let paused_after_first = run.iter().skip(1).all(|(marker, _)| marker.clause && marker.pause);
        // Two digits in a row are clear too: the model only writes digits when it hears counting.
        let digits = run.iter().filter(|(marker, _)| matches!(marker.kind, Kind::Number(_, Family::Digit))).count();
        let clear = all_strict
            || (!reads_as_quantities(pieces, run) && (run.len() >= 3 || paused_after_first || digits >= 2));
        if numbers >= 2 && clear {
            kept.append(run);
        } else {
            run.clear();
        }
    };
    // A clearer marker of the same number further on wins: in "primero, dos tazas de harina.
    // Segundo, ..." the item number is "segundo", and "dos" is a quantity.
    let clearer_later = |markers: &[Marker], index: usize, value: u8, this_family: Family| {
        markers[index + 1..]
            .iter()
            .take_while(|later| !matches!(later.kind, Kind::Number(later_value, _) if later_value > value))
            .any(|later| {
                later.strict
                    && matches!(later.kind, Kind::Number(later_value, later_family)
                        if later_value == value && counts_together(later_family, this_family))
            })
    };
    let weak_with_clearer_later: Vec<bool> = markers
        .iter()
        .enumerate()
        .map(|(index, marker)| match marker.kind {
            Kind::Number(value, this_family) => !marker.strict && clearer_later(&markers, index, value, this_family),
            _ => false,
        })
        .collect();
    for (index, marker) in markers.into_iter().enumerate() {
        match marker.kind {
            Kind::Bullet => {
                flush(&mut run, &mut kept);
                family = None;
                kept.push((marker, "- ".to_string()));
            }
            Kind::Number(value, this_family) => {
                let expected = run.len() as u8 + 1;
                let ended_by_last = run.last().is_some_and(|(marker, _)| marker.kind == Kind::Last);
                let continues = !run.is_empty()
                    && !ended_by_last
                    && family.is_some_and(|family| counts_together(family, this_family))
                    && value == expected;
                if continues && weak_with_clearer_later[index] {
                    // Not the item number: a clearer one comes later.
                } else if continues {
                    if has_words_between(pieces, &run.last().expect("the run has markers").0, &marker) {
                        run.push((marker, format!("{value}. ")));
                        if this_family != Family::Digit {
                            family = Some(this_family);
                        }
                    } else if marker.strict {
                        // Counting with nothing to count: "uno, dos, tres, probando".
                        run.clear();
                        family = None;
                    }
                } else if value == 1 && this_family == Family::Digit && run.len() == 1 && !run[0].0.strict {
                    // "... del one stop ... 1 lavar": the digit is where the counting starts.
                    run.clear();
                    family = Some(this_family);
                    run.push((marker, "1. ".to_string()));
                } else if !run.is_empty() && !marker.strict {
                    // A number inside an item, such as "compra uno", doesn't break the list.
                } else {
                    flush(&mut run, &mut kept);
                    if value == 1 {
                        family = Some(this_family);
                        run.push((marker, "1. ".to_string()));
                    } else {
                        family = None;
                    }
                }
            }
            Kind::Last => {
                let ended_by_last = run.last().is_some_and(|(marker, _)| marker.kind == Kind::Last);
                if run.len() >= 2 && !ended_by_last {
                    let value = run.len() + 1;
                    run.push((marker, format!("{value}. ")));
                }
            }
        }
    }
    flush(&mut run, &mut kept);
    kept.sort_by_key(|(marker, _)| marker.at);
    kept
}

/// Ends an item: no trailing comma, and a full stop only when the item is more than one word.
fn close_item(item: &mut [Piece]) {
    let words = item.iter().filter(|piece| matches!(piece, Piece::Word(_))).count();
    let Some(Piece::Word(last)) = item.iter_mut().rev().find(|piece| matches!(piece, Piece::Word(_))) else { return };
    let trimmed = last.trail.trim_end_matches([',', ';', ':']).to_string();
    last.trail = trimmed;
    if last.trail.contains(['?', '!', '…']) {
        return;
    }
    if words >= 2 {
        if !last.trail.contains('.') {
            last.trail.push('.');
        }
    } else {
        last.trail = last.trail.replace('.', "");
    }
}

/// A short sentence before a list introduces it, so it ends with a colon: "Necesito:",
/// "Ingredientes:". The model often writes the pause there as a full stop.
const LONGEST_INTRO: usize = 8;

fn introduce(out: &mut [Piece]) {
    let mut words = 0;
    for piece in out.iter().rev().skip(1) {
        match piece {
            Piece::Break(_) => break,
            Piece::Word(word) if is_hard_stop(&word.trail) => break,
            Piece::Word(_) => words += 1,
        }
    }
    let Some(Piece::Word(intro)) = out.last_mut() else { return };
    if intro.trail.contains(['?', '!', '…']) {
        return;
    }
    let short = words + 1 <= LONGEST_INTRO;
    if !is_hard_stop(&intro.trail) || (intro.trail == "." && short) {
        intro.trail = ":".to_string();
    }
}

/// Phrases that close the last item, so what follows goes on its own line: "... las acciones y ya
/// de esa manera ..." The words stay; only the line break is added.
const ENDINGS: &[&[&str]] = &[
    &["y", "ya"],
    &["y", "listo"],
    &["eso", "es", "todo"],
    &["eso", "seria", "todo"],
    &["and", "that's", "it"],
    &["and", "that’s", "it"],
    &["that's", "all"],
    &["that’s", "all"],
];

/// True when an ending phrase starts here. "Y ya que estás ahí" means "since", so it isn't one.
fn ending_at(pieces: &[Piece], at: usize) -> bool {
    ENDINGS.iter().any(|phrase| {
        phrase_at(pieces, at, phrase)
            && !(phrase[1] == "ya" && word_at(pieces, at + 2).is_some_and(|word| key(word) == "que"))
    })
}

/// Words that follow a repeated number but never a quantity: "tercero, tres que el responsable".
/// "Tercero, tres tazas de harina" keeps its "tres".
const AFTER_AN_ECHO: &[&str] = &[
    "que", "el", "la", "los", "las", "lo", "es", "son", "hay", "seria", "sea", "y", "de", "en", "a", "con", "para",
    "the", "that", "is", "to", "we", "and",
];

/// True when the word here repeats the item's number: "tercero, tres que ..." or "tres, tres que ...".
fn echoes_number(pieces: &[Piece], at: usize, value: u8, family: Family) -> bool {
    let Some(word) = word_at(pieces, at) else { return false };
    let same_number = COUNTING.iter().any(|&(phrase, this_value, this_family)| {
        phrase.len() == 1 && phrase[0] == key(word) && this_value == value && language(this_family) == language(family)
    });
    same_number && word_at(pieces, at + 1).is_some_and(|next| AFTER_AN_ECHO.contains(&key(next).as_str()))
}

/// Turns spoken lists into written ones. Text with no list comes back unchanged.
pub(crate) fn make_lists(pieces: Vec<Piece>) -> Vec<Piece> {
    let lists = real_lists(&pieces, find_markers(&pieces));
    if lists.is_empty() {
        return pieces;
    }
    let starts: std::collections::HashMap<usize, usize> =
        lists.iter().enumerate().map(|(index, (marker, _))| (marker.at, index)).collect();
    let dropped: std::collections::HashSet<usize> = lists.iter().filter_map(|(marker, _)| marker.joined).collect();

    let mut out: Vec<Piece> = Vec::with_capacity(pieces.len() + lists.len() * 2);
    // Where the item being written started in `out`, and whether it is the last of its list.
    let mut item_start: Option<usize> = None;
    let mut last_item = false;
    let mut label_next: Option<String> = None;
    // The number of the item just started, to drop it if said twice.
    let mut echo: Option<(u8, Family)> = None;
    let mut at = 0;
    while at < pieces.len() {
        if let Some(&index) = starts.get(&at) {
            let (marker, label) = &lists[index];
            if let Some(start) = item_start.take() {
                close_item(&mut out[start..]);
            } else {
                introduce(&mut out);
            }
            if !out.is_empty() && !matches!(out.last(), Some(Piece::Break(_))) {
                out.push(Piece::Break(LINE));
            }
            at += marker.length;
            let next_marker = lists.get(index + 1).map(|(next, _)| next.at);
            last_item = next_marker.is_none_or(|next| {
                // A line break between two markers ends the list there.
                pieces[at..next].iter().any(|piece| matches!(piece, Piece::Break(_)))
            });
            // The first word of the item carries the label.
            label_next = Some(label.clone());
            echo = match marker.kind {
                Kind::Number(value, family) if family != Family::Digit => Some((value, family)),
                _ => None,
            };
            continue;
        }
        if dropped.contains(&at) {
            at += 1;
            continue;
        }
        let piece = pieces[at].clone();
        at += 1;
        match &piece {
            Piece::Break(_) => {
                if let Some(start) = item_start.take() {
                    close_item(&mut out[start..]);
                }
                out.push(piece);
            }
            Piece::Word(word) => {
                if label_next.is_some()
                    && let Some((value, family)) = echo.take()
                    && echoes_number(&pieces, at - 1, value, family)
                {
                    continue;
                }
                let ends_sentence = is_hard_stop(&word.trail);
                let mut word = word.clone();
                if last_item && label_next.is_none() && ending_at(&pieces, at - 1) {
                    // "... las acciones y ya de esa manera ...": the list ends before "y ya".
                    if let Some(start) = item_start.take() {
                        close_item(&mut out[start..]);
                        out.push(Piece::Break(LINE));
                        word.core = capitalized(&word.core);
                    }
                }
                if let Some(label) = label_next.take() {
                    word.lead = format!("{label}{}", word.lead);
                    word.core = capitalized(&word.core);
                    item_start = Some(out.len());
                }
                out.push(Piece::Word(word));
                // The last item ends at its first full stop. Anything after it starts a new line.
                if last_item && ends_sentence && item_start.is_some() {
                    let following_marker = starts.contains_key(&at) || dropped.contains(&at);
                    if !following_marker {
                        if let Some(start) = item_start.take() {
                            close_item(&mut out[start..]);
                        }
                        if word_at(&pieces, at).is_some() {
                            out.push(Piece::Break(LINE));
                        }
                    }
                }
            }
        }
    }
    if let Some(start) = item_start {
        close_item(&mut out[start..]);
    }
    out
}

#[cfg(test)]
mod tests {
    use crate::tidy::tidy;

    fn check(cases: &[(&str, &str)]) {
        for (said, expected) in cases {
            assert_eq!(&tidy(said, None), expected, "for: {said}");
        }
    }

    fn unchanged(cases: &[&str]) {
        for said in cases {
            assert_eq!(&tidy(said, None), said, "for: {said}");
        }
    }

    #[test]
    fn counting_out_loud_makes_a_numbered_list() {
        check(&[
            (
                "Para mañana necesito, primero, comprar pan, segundo, llamar a Juan, y tercero, mandar el video.",
                "Para mañana necesito:\n1. Comprar pan.\n2. Llamar a Juan.\n3. Mandar el video.",
            ),
            ("Primero, comprar pan. Segundo, llamar a Juan.", "1. Comprar pan.\n2. Llamar a Juan."),
            ("Uno, harina. Dos, huevos. Tres, leche.", "1. Harina\n2. Huevos\n3. Leche"),
            (
                "Tengo tres cosas. Primero, el guion. Segundo, el casting. Y por último, la locación. Eso es todo.",
                "Tengo tres cosas:\n1. El guion.\n2. El casting.\n3. La locación.\nEso es todo.",
            ),
            (
                "En primer lugar, gracias a todos. En segundo lugar, bienvenidos.",
                "1. Gracias a todos.\n2. Bienvenidos",
            ),
            (
                "We need three things: first, a script, second, a camera, and third, a crew.",
                "We need three things:\n1. A script.\n2. A camera.\n3. A crew.",
            ),
            (
                "First of all, thank you. Second, welcome to the team. Finally, have fun.",
                "1. Thank you.\n2. Welcome to the team.\n3. Have fun.",
            ),
            ("1. Comprar pan. 2. Llamar a Juan.", "1. Comprar pan.\n2. Llamar a Juan."),
            (
                "Primera, la entrada. Segunda, el plato fuerte. Tercera, el postre.",
                "1. La entrada.\n2. El plato fuerte.\n3. El postre.",
            ),
            // Written the way Parakeet really wrote spoken lists, with invented words.
            (
                "hola ok bueno necesito hacer una lista primero regar las plantas y dos pasear al perro de nuevo y tres ver si ya llegó el paquete",
                "hola ok bueno necesito hacer una lista:\n1. Regar las plantas.\n2. Pasear al perro de nuevo.\n3. Ver si ya llegó el paquete.",
            ),
            (
                "A ver, lo que necesito son dos cosas, uno comprar las flores y dos, pintar la cerca.",
                "A ver, lo que necesito son dos cosas:\n1. Comprar las flores.\n2. Pintar la cerca.",
            ),
            (
                "Primero, comprar pan, dos, llamar a Juan, tres, mandar el video.",
                "1. Comprar pan.\n2. Llamar a Juan.\n3. Mandar el video.",
            ),
            (
                "A ver entonces vamos a hacer la lista que cosas que tengo que hacer hoy 1 lavar las toallas del one stop del centro 2 hornear el pastel grande 6 capas Y 3 tratar de arreglar la bici para Luna Y ya Eso es todo por hoy",
                "A ver entonces vamos a hacer la lista que cosas que tengo que hacer hoy:\n1. Lavar las toallas del one stop del centro.\n2. Hornear el pastel grande 6 capas.\n3. Tratar de arreglar la bici para Luna.\nY ya Eso es todo por hoy",
            ),
            (
                "Entonces, para los cambios que hay que hacer en la casa nueva tendría que ser 1 buscar mejores plantas para el jardín y ver si ellas son posibles causas del poco verde. Dos, el patio trasero revisar que los tubos estén bien sellados. Y tres que el encargado de la mudanza sea una persona que está capacitada para llevar los muebles a cabo. Nueva línea. Y ahí se acabó la lista.",
                "Entonces, para los cambios que hay que hacer en la casa nueva tendría que ser:\n1. Buscar mejores plantas para el jardín y ver si ellas son posibles causas del poco verde.\n2. El patio trasero revisar que los tubos estén bien sellados.\n3. Que el encargado de la mudanza sea una persona que está capacitada para llevar los muebles a cabo.\nY ahí se acabó la lista.",
            ),
            ("1 comprar pan 2 llamar a Juan", "1. Comprar pan.\n2. Llamar a Juan."),
            ("Primero, compra uno. Segundo, llama a Ana.", "1. Compra uno.\n2. Llama a Ana."),
            (
                "first check the cable and second restart it and third call me",
                "1. Check the cable.\n2. Restart it.\n3. Call me.",
            ),
        ]);
    }

    #[test]
    fn saying_bullet_makes_a_bulleted_list() {
        check(&[
            ("Ingredientes, viñeta, harina, viñeta, huevos, viñeta, leche.", "Ingredientes:\n- Harina\n- Huevos\n- Leche"),
            ("Ingredientes. Viñeta. Harina. Viñeta. Huevos.", "Ingredientes:\n- Harina\n- Huevos"),
            (
                "Shopping list Bullet Point Apples Bullet Point Bread Bullet Point Milk",
                "Shopping list:\n- Apples\n- Bread\n- Milk",
            ),
            (
                "Hoy fue un día muy largo y lleno de reuniones con el equipo. Viñeta. Revisar notas. Viñeta. Llamar a Ana.",
                "Hoy fue un día muy largo y lleno de reuniones con el equipo.\n- Revisar notas.\n- Llamar a Ana.",
            ),
            ("Viñeta, comprar pan para mañana.", "- Comprar pan para mañana."),
            (
                "Shopping list, bullet point, apples, bullet point, bread. Thanks.",
                "Shopping list:\n- Apples\n- Bread\nThanks.",
            ),
            ("To do: bullet, call Ana, bullet, send the cut.", "To do:\n- Call Ana.\n- Send the cut."),
        ]);
    }

    #[test]
    fn ordinary_sentences_with_the_same_words_stay() {
        unchanged(&[
            "Primero voy a comer y luego me duermo.",
            "First I want to thank you all.",
            "Compré dos panes y tres leches.",
            "Llegó en segundo lugar.",
            "Un segundo, ya voy.",
            "Esta viñeta del cómic es muy buena.",
            "The bullet train is fast.",
            "A bullet hit the wall.",
            "The bullet point is clear.",
            "Tengo uno, pero quiero dos.",
            "Son las dos, tres personas llegaron tarde.",
            "Segundo, no me gusta.",
            "El primero, el segundo y el tercero llegaron.",
            "Por último, gracias.",
            "Finally, we made it.",
            "Uno, dos, tres, probando.",
            "Compré 1 kilo, 2 litros y 3 latas.",
            "Compré 1 pan, dos leches y tres huevos.",
            "Tengo 1 hijo y 2 hijas.",
            "Vi 1 perro 2 veces en el parque.",
            "El 1 de mayo, el 2 de junio y el 3 de julio salimos.",
            "Capítulo 1 habla de perros, capítulo 2 habla de gatos y capítulo 3 habla de aves.",
            "A las 2 comemos y a las 3 dormimos.",
            "Primero, comprar 2 panes y luego dormir.",
            "Primero voy a comer y dos horas después me duermo.",
            "Primero, quiero agradecer. Y dos personas vinieron.",
            "Es uno de los mejores y dos de ellos ganaron.",
            "Primero comimos y segundo nos fuimos.",
        ]);
    }

    /// Prints every saved text that would turn into a list. Run by hand with MIMI_LIST_TEXTS set
    /// to a file with one text per line.
    #[test]
    #[ignore]
    fn saved_texts_that_become_lists() {
        let Ok(path) = std::env::var("MIMI_LIST_TEXTS") else { return };
        let texts = std::fs::read_to_string(path).expect("the texts file");
        let mut changed = 0;
        let mut total = 0;
        for text in texts.lines().filter(|line| !line.trim().is_empty()) {
            total += 1;
            let cleaned = tidy(text, None);
            let listed = cleaned.lines().any(|line| {
                line.starts_with("- ") || line.split_once(". ").is_some_and(|(n, _)| n.parse::<u8>().is_ok())
            });
            let already = text.contains("- ") || text.contains("1. ");
            if listed && !already {
                changed += 1;
                println!("LIST: {text}\n  ->  {}", cleaned.replace('\n', " | "));
            }
        }
        println!("{changed} of {total} texts became lists");
    }

    /// However the model writes the same spoken list, it has to come out the same.
    #[test]
    fn every_way_the_model_writes_a_list() {
        let styles = [["primero", "segundo", "tercero"], ["uno", "dos", "tres"], ["1", "2", "3"], ["1", "dos", "tres"]];
        let title = |text: &str, wanted: bool| if wanted { crate::tidy::capitalized(text) } else { text.to_string() };
        let mut tried = 0;
        for (style, numbers) in styles.iter().enumerate() {
            for pause in ["", ","] {
                for between in ["", ",", "."] {
                    // Plain words with no punctuation at all and no "y" are too much like ordinary talk.
                    if (style == 1 || style == 3) && pause.is_empty() && between.is_empty() {
                        continue;
                    }
                    for and in [false, true] {
                        let stop = between == ".";
                        let third = if and {
                            format!("{} {}", title("y", stop), numbers[2])
                        } else {
                            title(numbers[2], stop)
                        };
                        let said = format!(
                            "Tengo que hacer esto hoy{between} {}{pause} revisar el guion{between} {}{pause} llamar al equipo{between} {third}{pause} mandar el video.",
                            title(numbers[0], stop),
                            title(numbers[1], stop),
                        );
                        assert_eq!(
                            tidy(&said, None),
                            "Tengo que hacer esto hoy:\n1. Revisar el guion.\n2. Llamar al equipo.\n3. Mandar el video.",
                            "for: {said}"
                        );
                        tried += 1;
                    }
                }
            }
        }
        assert_eq!(tried, 44);
    }

    #[test]
    fn saying_how_the_list_ends() {
        check(&[
            (
                "Necesito, primero, revisar el guion, segundo, llamar al equipo y ya de esa manera arrancamos.",
                "Necesito:\n1. Revisar el guion.\n2. Llamar al equipo.\nY ya de esa manera arrancamos.",
            ),
            (
                "Necesito, primero, revisar el guion, segundo, llamar al equipo, punto y aparte, de esa manera arrancamos.",
                "Necesito:\n1. Revisar el guion.\n2. Llamar al equipo.\n\nDe esa manera arrancamos.",
            ),
            (
                "Necesito, primero, revisar el guion, segundo, llamar al equipo, nueva línea, de esa manera arrancamos.",
                "Necesito:\n1. Revisar el guion.\n2. Llamar al equipo.\nDe esa manera arrancamos.",
            ),
            (
                "Uno, harina. Dos, huevos y listo con eso basta.",
                "1. Harina\n2. Huevos\nY listo con eso basta.",
            ),
            (
                "We need, first, a script, second, a camera and that's it for today.",
                "We need:\n1. A script.\n2. A camera.\nAnd that's it for today.",
            ),
            // "Y ya que" means "since", so the item goes on.
            (
                "Primero, llamar a Juan. Segundo, ir al banco y ya que estás ahí pagar la luz.",
                "1. Llamar a Juan.\n2. Ir al banco y ya que estás ahí pagar la luz.",
            ),
        ]);
    }

    #[test]
    fn a_number_said_twice_is_written_once() {
        check(&[
            (
                "Primero, revisar el guion. Segundo, llamar al equipo. Tercero, tres que el responsable firme.",
                "1. Revisar el guion.\n2. Llamar al equipo.\n3. Que el responsable firme.",
            ),
            (
                "Uno, revisar el guion. Dos, dos, el equipo de sonido.",
                "1. Revisar el guion.\n2. El equipo de sonido.",
            ),
            // A quantity stays.
            (
                "Primero, dos tazas de harina. Segundo, dos huevos. Tercero, tres cucharadas de azúcar.",
                "1. Dos tazas de harina.\n2. Dos huevos.\n3. Tres cucharadas de azúcar.",
            ),
        ]);
    }

    #[test]
    fn lists_work_with_the_other_rules() {
        check(&[
            (
                "Eh, necesito, primero, eh, comprar pan, segundo, llamar a a Juan.",
                "Necesito:\n1. Comprar pan.\n2. Llamar a Juan.",
            ),
            (
                "Hola Juan, nueva línea, primero, el guion, segundo, el casting.",
                "Hola Juan,\n1. El guion.\n2. El casting.",
            ),
        ]);
    }
}
