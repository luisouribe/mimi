//! Rule-based cleanup of a dictation, with no AI: removes filler sounds, collapses stutters,
//! turns spoken commands into line breaks and spoken lists into written ones (see `lists.rs`). When in doubt, a rule leaves the words alone, because
//! a stutter left behind is a smaller problem than a word the person meant being deleted.
//!
//! Spoken commands rely on punctuation. Every speech model writes a pause around a command as a
//! full stop or commas ("Hola Juan, nueva línea, gracias."), while "una nueva línea de productos"
//! has none. Word timings aren't used, because Parakeet's show no gap at a pause and Whisper has none.

/// One written word, with the punctuation stuck to each side of it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Word {
    pub(crate) lead: String,
    pub(crate) core: String,
    pub(crate) trail: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Piece {
    Word(Word),
    Break(&'static str),
}

pub(crate) const LINE: &str = "\n";
const PARAGRAPH: &str = "\n\n";

/// Each phrase is written in lowercase with no accents. The last value marks a command that
/// also ends the sentence before it, as "punto y aparte" means in Spanish.
const COMMANDS: &[(&[&str], &str, bool)] = &[
    (&["new", "line"], LINE, false),
    (&["newline"], LINE, false),
    (&["line", "break"], LINE, false),
    (&["new", "paragraph"], PARAGRAPH, false),
    (&["nueva", "linea"], LINE, false),
    (&["salto", "de", "linea"], LINE, false),
    (&["nuevo", "parrafo"], PARAGRAPH, false),
    (&["punto", "y", "aparte"], PARAGRAPH, true),
];

/// Words people repeat on purpose, for emphasis, in sounds or when reading out numbers.
const KEEP_REPEATED: &[&str] = &[
    "very", "really", "so", "too", "real", "super", "pretty", "much", "many", "more", "long", "far", "big", "little",
    "bad", "good", "high", "never", "ever", "always", "now", "quick", "fast", "run", "go", "come", "wait", "stop",
    "please", "thanks", "yes", "yeah", "yep", "no", "nope", "ok", "okay", "well", "right", "oh", "wow", "hey", "hello",
    "hi", "bye", "haha", "knock", "chop", "boo", "choo", "night", "there", "hear", "tut", "muy", "mas", "mucho", "mucha", "muchos", "muchas", "tan", "tanto", "bien", "mal", "poco", "poquito",
    "casi", "nunca", "siempre", "nada", "todo", "ya", "ahora", "rapido", "corre", "ven", "dale", "vamos", "espera",
    "gracias", "si", "claro", "vale", "bueno", "ay", "ah", "hola", "adios", "ciao", "chao",
];

/// Phone numbers and laughter, where any count can be real.
const KEEP_ANY_COUNT: &[&str] = &[
    "ha", "he", "ja", "je", "ji", "jo", "bla", "blah", "pum", "toc", "tac", "tic", "tick", "zero", "one", "two", "three",
    "four", "five", "six", "seven", "eight", "nine", "ten", "cero", "uno", "dos", "tres", "cuatro", "cinco", "seis",
    "siete", "ocho", "nueve", "diez",
];

/// Parakeet can get stuck and write a repeated word many times: "muy muy" came out as "muy" 11 times.
/// From this many, a word that's fine when repeated goes back to two.
const STUCK_REPEATS: usize = 4;

/// Doubles that can be correct grammar, as in "I think that that works". Three or more are a stutter.
const FINE_TWICE: &[&str] = &["that", "had", "is"];

/// Words people trip over. A word said twice only collapses if it's here, because any other
/// double may be meant, as in "perdón perdón" or "mira mira". Three or more always collapse,
/// unless the word is in `KEEP_REPEATED`. Written in lowercase with no accents.
const TRIPPED_OVER: &[&str] = &[
    "i", "we", "you", "she", "it", "they", "i'm", "it's", "don't", "that's", "the", "a", "an", "but", "if", "because",
    "when", "to", "of", "in", "on", "at", "for", "with", "my", "this", "think", "want", "have", "can", "was", "were",
    "would", "will", "did", "do", "am", "are", "yo", "tu", "el", "ella", "nosotros", "la", "los", "las", "lo", "un",
    "una", "unos", "unas", "pero", "porque", "cuando", "que", "de", "del", "al", "en", "con", "por", "para", "mi", "su",
    "me", "te", "se", "le", "nos", "es", "esta", "estoy", "soy", "son", "hay", "va", "voy", "tengo", "puedo", "quiero",
    "creo",
];

/// A repeated group starting with one of these is a false start, as in "I think I think"
/// or "estoy probando estoy probando".
const FALSE_STARTS: &[&str] = &[
    "i", "we", "you", "she", "it", "they", "i'm", "yo", "ella", "nosotros", "estoy", "soy", "voy", "tengo", "puedo",
    "quiero", "creo",
];

/// A short opening starting with one of these is a greeting, where a comma before a line break is right.
const GREETINGS: &[&str] = &[
    "hola", "hi", "hello", "hey", "dear", "good", "greetings", "estimado", "estimada", "estimados", "estimadas",
    "querido", "querida", "queridos", "queridas", "apreciado", "apreciada", "buenos", "buenas", "saludos",
];
const LONGEST_GREETING: usize = 5;
/// A clause this short, with "and" or "y" in it, is the end of a list rather than a new sentence.
const LONGEST_LIST_END: usize = 5;

/// Groups joined by these are usually a figure of speech, as in "on and on and on".
const JOINING_WORDS: &[&str] = &["and", "or", "y", "e", "o", "u"];

pub(crate) fn fold(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'ä' | 'â' => 'a',
            'é' | 'è' | 'ë' | 'ê' => 'e',
            'í' | 'ì' | 'ï' | 'î' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' => 'o',
            'ú' | 'ù' | 'ü' | 'û' => 'u',
            other => other,
        })
        .collect()
}

pub(crate) fn split_word(raw: &str) -> Word {
    let start = raw.find(char::is_alphanumeric).unwrap_or(raw.len());
    let end = raw.rfind(char::is_alphanumeric).map_or(start, |at| at + raw[at..].chars().next().map_or(0, char::len_utf8));
    Word {
        lead: raw[..start].to_string(),
        core: raw[start..end].to_string(),
        trail: raw[end..].to_string(),
    }
}

pub(crate) fn is_hard_stop(trail: &str) -> bool {
    trail.contains(['.', '!', '?', '…', '。', '\u{FF01}', '\u{FF1F}'])
}

fn is_soft_stop(trail: &str) -> bool {
    !is_hard_stop(trail) && trail.contains([',', ';', ':', '、'])
}

pub(crate) fn capitalized(text: &str) -> String {
    let mut letters = text.chars();
    letters.next().map_or_else(String::new, |first| first.to_uppercase().chain(letters).collect())
}

/// "Tienda" becomes "tienda", but "ONU" stays as it is.
fn lowercased(text: &str) -> String {
    if text.chars().filter(|c| c.is_alphabetic()).all(char::is_uppercase) && text.chars().count() > 1 {
        return text.to_string();
    }
    let mut letters = text.chars();
    letters.next().map_or_else(String::new, |first| first.to_lowercase().chain(letters).collect())
}

/// Words a sentence can't end on, so a period after them is a hesitation written down:
/// "Fui a la. Este. Tienda" and "Pues este. No sé".
fn cannot_end_sentence(word: &Word) -> bool {
    let key = fold(&word.core);
    NEVER_BEFORE_THIS.contains(&key.as_str()) || BEFORE_FILLER_ESTE.contains(&key.as_str())
}

fn starts_upper(text: &str) -> bool {
    text.chars().next().is_some_and(char::is_uppercase)
}

/// How firmly punctuation separates a phrase from what's around it.
#[derive(PartialEq, Clone, Copy)]
pub(crate) enum Edge {
    Open,
    Soft,
    Hard,
}

fn edge_after(trail: &str) -> Edge {
    if trail.contains(['?', '\u{FF1F}']) {
        Edge::Open
    } else if is_hard_stop(trail) {
        Edge::Hard
    } else if is_soft_stop(trail) {
        Edge::Soft
    } else {
        Edge::Open
    }
}

/// The sentence so far, up to but not including this word.
fn sentence_before(words: &[Word], at: usize) -> &[Word] {
    let start = words[..at].iter().rposition(|word| is_hard_stop(&word.trail)).map_or(0, |stop| stop + 1);
    &words[start..at]
}

fn is_greeting(sentence: &[Word]) -> bool {
    sentence.len() <= LONGEST_GREETING
        && sentence.first().is_some_and(|word| GREETINGS.contains(&fold(&word.core).as_str()))
}

fn is_joining_word(word: &Word) -> bool {
    JOINING_WORDS.contains(&fold(&word.core).as_str())
}

/// True when commas around a phrase look like a list, as in "camisas, nueva línea, nuevo logo y web".
fn looks_like_a_list(words: &[Word], at: usize, length: usize) -> bool {
    let sentence = sentence_before(words, at);
    if sentence.last().is_some_and(|word| word.trail.contains([':', ';'])) {
        return true;
    }
    let rest = &words[at + length..];
    if rest.first().is_some_and(is_joining_word) {
        return true;
    }
    let clause_after = rest.iter().position(|word| !word.trail.is_empty()).map_or(rest.len(), |end| end + 1);
    if clause_after <= LONGEST_LIST_END && rest[..clause_after].iter().any(is_joining_word) {
        return true;
    }
    // Two short items in a row: "camisas, pantalones, nueva línea".
    let before_last = &sentence[..sentence.len().saturating_sub(1)];
    let last_clause = before_last.iter().rev().take_while(|word| word.trail.is_empty()).count() + 1;
    last_clause <= 2 && before_last.iter().any(|word| is_soft_stop(&word.trail)) && !is_greeting(sentence)
}

/// How firmly the word before a phrase separates it. The start of a line counts as firm.
pub(crate) fn edge_before(previous: Option<&Word>) -> Edge {
    match previous {
        None => Edge::Hard,
        Some(word) if is_hard_stop(&word.trail) => Edge::Hard,
        Some(word) => edge_after(&word.trail),
    }
}

/// True when punctuation shows that the words at this spot were said on their own.
/// `never_a_list` skips the check for lists, for phrases that can't be an item in one.
pub(crate) fn said_on_its_own(words: &[Word], at: usize, length: usize, before: Edge, never_a_list: bool) -> bool {
    let Some(found) = words.get(at..at + length) else { return false };
    let Some(last) = found.last() else { return false };
    let inside_is_clean = found.iter().enumerate().all(|(index, word)| {
        word.lead.is_empty() && (index + 1 == found.len() || word.trail.is_empty())
    });
    if !inside_is_clean {
        return false;
    }
    // A capital right after a lowercase phrase starts a new sentence: "nueva línea Gracias".
    // "New Line Cinema" keeps its own capitals, so it isn't taken for a command.
    let next_starts_sentence = !starts_upper(&last.core)
        && words.get(at + length).is_some_and(|next| starts_upper(&next.core));
    let after = if last.trail.is_empty() && (at + length == words.len() || next_starts_sentence) {
        Edge::Hard
    } else {
        edge_after(&last.trail)
    };
    before != Edge::Open
        && after != Edge::Open
        && (before == Edge::Hard || after == Edge::Hard || never_a_list || !looks_like_a_list(words, at, length))
}

/// True if these words, without capitals or accents, are a spoken command.
pub(crate) fn is_command(words: &[String]) -> bool {
    COMMANDS.iter().any(|(phrase, _, _)| phrase.len() == words.len() && phrase.iter().zip(words).all(|(a, b)| a == b))
}

/// The command starting at this word, if the punctuation shows it was said on its own.
fn command_at(words: &[Word], at: usize, before: Edge) -> Option<(usize, &'static str, bool)> {
    COMMANDS.iter().find_map(|&(phrase, text, ends_sentence)| {
        let found = words.get(at..at + phrase.len())?;
        let same = found.iter().zip(phrase).all(|(word, part)| fold(&word.core) == *part);
        // "Punto y aparte" is never part of a list.
        (same && said_on_its_own(words, at, phrase.len(), before, ends_sentence)).then_some((phrase.len(), text, ends_sentence))
    })
}

fn apply_commands(words: Vec<Word>) -> Vec<Piece> {
    let mut pieces: Vec<Piece> = Vec::with_capacity(words.len());
    let mut at = 0;
    while at < words.len() {
        let before = match pieces.last() {
            Some(Piece::Word(word)) => edge_before(Some(word)),
            _ => Edge::Hard,
        };
        if let Some((length, text, ends_sentence)) = command_at(&words, at, before) {
            // The comma before a command usually marks the pause, not the sentence.
            let keep_comma = !ends_sentence && is_greeting(sentence_before(&words, at));
            if let Some(Piece::Word(previous)) = pieces.last_mut() {
                let ends_clause = previous.trail == "," || previous.trail == ";" || previous.trail == "、";
                if (ends_sentence && !is_hard_stop(&previous.trail)) || (ends_clause && !keep_comma) {
                    previous.trail = ".".to_string();
                }
            }
            pieces.push(Piece::Break(text));
            at += length;
        } else {
            pieces.push(Piece::Word(words[at].clone()));
            at += 1;
        }
    }
    pieces
}

fn all_same(text: &str, letter: char) -> bool {
    !text.is_empty() && text.chars().all(|c| c == letter)
}

/// True for sounds that are never real words, in English or Spanish.
fn is_filler_sound(word: &Word) -> bool {
    let core = word.core.to_lowercase();
    let run = |first: char, second: char| {
        let rest = core.trim_start_matches(first);
        rest.len() < core.len() && all_same(rest, second)
    };
    match core.as_str() {
        // These two also appear in real text ("an em dash"), so they need punctuation around them.
        "er" | "em" => !word.trail.is_empty(),
        "erm" | "ehm" => true,
        _ => run('u', 'm') || run('u', 'h') || run('h', 'm') || run('e', 'h') || run('e', 'm') || (core.len() >= 2 && all_same(&core, 'm')) || (core.len() >= 3 && all_same(&core, 'e')),
    }
}

/// Words after which a Spanish "este" is still a hesitation, as in "pues este, no sé".
const BEFORE_FILLER_ESTE: &[&str] = &["pues", "bueno", "entonces"];
/// Words that "este" meaning "this" can never follow, as in "fui a la... este tienda".
/// "El" isn't here, because "el este" is the direction east.
const NEVER_BEFORE_THIS: &[&str] = &["la", "los", "las", "un", "una", "unos", "unas", "mi", "tu", "su", "mis", "tus", "sus"];

/// Words that "este" meaning "this" never comes right before, so when no pause is written,
/// the next word still shows a hesitation: "Este quería preguntarte algo", "Mimi este ahora vamos".
/// Words that also follow "this" are left out: "es", "no", "se", "ya", "también", nouns.
const NEVER_AFTER_THIS: &[&str] = &[
    // Articles and people
    "el", "los", "un", "una", "unos", "unas", "yo", "tu", "ella", "ellos", "ellas", "nosotros", "nosotras", "usted",
    "ustedes", "vosotros",
    // Words that start a new thought
    "pues", "bueno", "entonces", "ahora", "mira", "oye", "digamos",
    // "I" and "we" verbs, which "this" can't be the subject of, plus a few that people start with after a pause
    "queria", "quiero", "quisiera", "creo", "creia", "pienso", "pensaba", "necesito", "necesitaba", "tengo", "tenia",
    "voy", "iba", "estoy", "estaba", "digo", "decia", "dije", "fui", "hice", "puedo", "podia", "sabia", "vengo",
    "estamos", "vamos", "tenemos", "queremos", "podemos", "necesitamos", "hicimos", "fuimos",
];

/// Verbs whose object can be "este" followed by one of those words, as in "quiero este ahora".
const TAKES_THIS: &[&str] = &[
    "quiero", "quieres", "quiere", "queremos", "dame", "deme", "denme", "prefiero", "prefieres", "compre", "compro",
    "compra", "uso", "usa", "usamos", "toma", "tomo", "mira", "mire", "ves", "veo", "tengo", "tienes", "tiene",
    "necesito", "elijo", "escojo", "gusta", "gusto", "encanta", "vendo", "traje", "trae", "traigo", "pasame", "agarra",
    "agarro", "llevo", "llevate", "hago", "haz", "termina", "termine", "termino", "mando", "envio", "revisa", "reviso",
    "lee", "lei", "leo", "escribe",
];

fn trails_off(trail: &str) -> bool {
    trail.contains('…') || trail.ends_with("..")
}

fn is_este(word: &Word) -> bool {
    let core = fold(&word.core);
    core.starts_with("est") && core.len() >= 4 && all_same(&core[3..], 'e') && word.lead.is_empty()
}

/// Marks each "este" that is the Spanish hesitation, as in "Este, quería preguntarte algo",
/// rather than the word "this", as in "este libro" or "quiero este, no ese".
/// It has to be cut off by a comma or "...", start a sentence or follow a comma, and not be part of a list.
/// It also counts when it stands alone as a sentence before more words, as in "Este. Quería saber",
/// or follows a word that "this" can't, as in "fui a la... este tienda".
/// A drawn-out "esteee" is always a hesitation. A run, as in "este este,", is judged as one.
fn filler_este(pieces: &[Piece]) -> Vec<bool> {
    let mut marked = vec![false; pieces.len()];
    let mut index = 0;
    while index < pieces.len() {
        let Piece::Word(word) = &pieces[index] else {
            index += 1;
            continue;
        };
        if !is_este(word) {
            index += 1;
            continue;
        }
        if fold(&word.core).len() > 4 {
            marked[index] = !word.trail.contains(['?', '!']);
            index += 1;
            continue;
        }
        let mut last = index;
        while let (Piece::Word(current), Some(Piece::Word(next))) = (&pieces[last], pieces.get(last + 1)) {
            if current.trail.is_empty() && is_este(next) {
                last += 1;
            } else {
                break;
            }
        }
        if este_was_a_pause(pieces, index, last) {
            marked[index..=last].fill(true);
        }
        index = last + 1;
    }
    marked
}

fn este_was_a_pause(pieces: &[Piece], first: usize, last: usize) -> bool {
    let Piece::Word(end) = &pieces[last] else { return false };
    if end.trail.contains(['?', '!']) {
        return false;
    }
    let previous = match first.checked_sub(1).map(|at| &pieces[at]) {
        Some(Piece::Word(previous)) => Some(previous),
        _ => None,
    };
    let after_article = previous.is_some_and(|previous| {
        let key = fold(&previous.core);
        (previous.trail.is_empty() || trails_off(&previous.trail) || previous.trail == ".")
            && (NEVER_BEFORE_THIS.contains(&key.as_str()) || (key == "el" && trails_off(&previous.trail)))
    });
    if after_article {
        return true;
    }
    // "Este. Quería saber" is a pause, while "¿Cuál? Este." answers a question.
    let more_follows = matches!(pieces.get(last + 1), Some(Piece::Word(_)));
    let opens_sentence =
        previous.is_none_or(|previous| is_hard_stop(&previous.trail) && !previous.trail.contains(['?', '\u{FF1F}']));
    if end.trail == "." && opens_sentence && more_follows {
        return true;
    }
    // "Pues este. No sé qué decirte."
    if end.trail == "." && more_follows && previous.is_some_and(|previous| {
        previous.trail.is_empty() && BEFORE_FILLER_ESTE.contains(&fold(&previous.core).as_str())
    }) {
        return true;
    }
    if end.trail.is_empty()
        && let Some(Piece::Word(next)) = pieces.get(last + 1)
        && next.lead.is_empty()
    {
        let object_of = previous.is_some_and(|previous| {
            previous.trail.is_empty() && TAKES_THIS.contains(&fold(&previous.core).as_str())
        });
        if (NEVER_AFTER_THIS.contains(&fold(&next.core).as_str()) || is_filler_sound(next)) && !object_of {
            return true;
        }
    }
    let cut_off = end.trail == "," || trails_off(&end.trail);
    let after_pause = match first.checked_sub(1).map(|at| &pieces[at]) {
        None | Some(Piece::Break(_)) => true,
        Some(Piece::Word(previous)) => {
            is_hard_stop(&previous.trail)
                || is_soft_stop(&previous.trail)
                || (previous.trail.is_empty() && BEFORE_FILLER_ESTE.contains(&fold(&previous.core).as_str()))
        }
    };
    if !cut_off || !after_pause {
        return false;
    }
    // "Ese, este y aquel" is a list of things, so "este" is meant.
    let start = pieces[..first].iter().rposition(|piece| matches!(piece, Piece::Break(_))).map_or(0, |at| at + 1);
    let stop = pieces[first..].iter().position(|piece| matches!(piece, Piece::Break(_))).map_or(pieces.len(), |at| first + at);
    let stretch: Vec<Word> = pieces[start..stop]
        .iter()
        .filter_map(|piece| match piece {
            Piece::Word(word) => Some(word.clone()),
            Piece::Break(_) => None,
        })
        .collect();
    !looks_like_a_list(&stretch, first - start, last - first + 1)
}

fn starts_sentence(kept: &[Piece]) -> bool {
    match kept.last() {
        None | Some(Piece::Break(_)) => true,
        Some(Piece::Word(word)) => is_hard_stop(&word.trail),
    }
}

fn opens_with_mark(word: &Word) -> bool {
    word.lead.contains(['¿', '¡'])
}

fn remove_fillers(pieces: Vec<Piece>) -> Vec<Piece> {
    let mut kept: Vec<Piece> = Vec::with_capacity(pieces.len());
    let mut capitalize_next = false;
    let mut lowercase_next = false;
    let este = filler_este(&pieces);
    for (index, piece) in pieces.iter().enumerate() {
        let word = match piece {
            Piece::Break(_) => {
                capitalize_next = false;
                lowercase_next = false;
                kept.push(piece.clone());
                continue;
            }
            Piece::Word(word) => word,
        };
        // "¿eh?" and "¡eh!" are said on purpose.
        let on_purpose = opens_with_mark(word) || word.trail.contains(['?', '!', '\u{FF1F}', '\u{FF01}']);
        if !(is_filler_sound(word) || este[index]) || on_purpose {
            let mut word = word.clone();
            if capitalize_next {
                word.core = capitalized(&word.core);
                capitalize_next = false;
            }
            if lowercase_next {
                word.core = lowercased(&word.core);
                lowercase_next = false;
            }
            kept.push(Piece::Word(word));
            continue;
        }

        let opened_sentence = starts_sentence(&kept);
        let next = match pieces.get(index + 1) {
            Some(Piece::Word(next)) => Some(next),
            _ => None,
        };
        // "Fui a la, este... tienda" trails off mid-sentence, so the dots don't end it.
        let mid_sentence = este[index] && next.is_some_and(|next| !starts_upper(&next.core));
        // The model wrote the hesitation as sentence breaks: "Fui a la. Este. Tienda" is one sentence.
        if este[index]
            && word.trail == "."
            && next.is_some_and(|next| next.lead.is_empty())
            && let Some(Piece::Word(previous)) = kept.last_mut()
            && (previous.trail.is_empty() || previous.trail == ".")
            && cannot_end_sentence(previous)
        {
            previous.trail.clear();
            lowercase_next = true;
            capitalize_next = false;
            continue;
        }
        if let Some(Piece::Word(previous)) = kept.last_mut() {
            if mid_sentence && trails_off(&previous.trail) {
                // The dots before it were the same hesitation: "fui a la... este tienda".
                previous.trail.clear();
            } else if is_hard_stop(&word.trail) && !mid_sentence {
                if !is_hard_stop(&previous.trail) {
                    previous.trail = ".".to_string();
                }
            } else if is_soft_stop(&previous.trail)
                // "Mimi, este ahora vamos" keeps its comma, because nothing closes the pause after "este".
                && (is_soft_stop(&word.trail) || trails_off(&word.trail) || !este[index])
                && !next.is_some_and(opens_with_mark)
                // A comma kept before a line break stays: "Hola Juan, eh, nueva línea".
                && !matches!(pieces.get(index + 1), Some(Piece::Break(_)))
            {
                // Both commas belonged to the filler: "I think, um, that".
                previous.trail.clear();
            }
        }
        capitalize_next = opened_sentence && (starts_upper(&word.core) || capitalize_next);
    }
    kept
}

fn is_plain_word(core: &str) -> bool {
    !core.is_empty() && core.chars().all(|c| (c.is_alphabetic() && c < '\u{250}') || c == '\'')
}

/// True when nothing but an optional comma separates this word from the next one.
fn runs_into_next(word: &Word, next: &Word) -> bool {
    next.lead.is_empty() && (word.trail.is_empty() || word.trail == ",")
}

/// Collapses one word said over and over.
fn collapse_runs(words: Vec<Word>) -> Vec<Word> {
    let mut result: Vec<Word> = Vec::with_capacity(words.len());
    let mut at = 0;
    while at < words.len() {
        let first = &words[at];
        let key = fold(&first.core);
        let mut loose_end = at + 1;
        while loose_end < words.len() && fold(&words[loose_end].core) == key && runs_into_next(&words[loose_end - 1], &words[loose_end]) {
            loose_end += 1;
        }
        // Accents can change the meaning ("él el", "sí si"), so two of them only count when identical.
        let spelled_same = words[at..loose_end].iter().all(|word| word.core.to_lowercase() == first.core.to_lowercase());
        let end = if loose_end - at >= 3 || spelled_same { loose_end } else { at + 1 };
        let count = end - at;
        let has_comma = words[at..end - 1].iter().any(|word| word.trail == ",");
        let tripped_over = TRIPPED_OVER.contains(&key.as_str());
        let fine_when_repeated = KEEP_REPEATED.contains(&key.as_str()) || (has_comma && !tripped_over);
        let stuck = count >= STUCK_REPEATS;
        let keep_all = count < 2
            || !is_plain_word(&first.core)
            || KEEP_ANY_COUNT.contains(&key.as_str())
            || (fine_when_repeated && !stuck)
            || (count == 2 && (FINE_TWICE.contains(&key.as_str()) || !tripped_over));
        if keep_all {
            result.extend_from_slice(&words[at..end]);
        } else {
            let last = &words[end - 1];
            let core = if starts_upper(&first.core) { capitalized(&last.core) } else { last.core.clone() };
            if FINE_TWICE.contains(&key.as_str()) || fine_when_repeated {
                result.push(Word { lead: first.lead.clone(), core: core.clone(), trail: first.trail.clone() });
                result.push(Word { lead: String::new(), core: last.core.clone(), trail: last.trail.clone() });
            } else {
                result.push(Word { lead: first.lead.clone(), core, trail: last.trail.clone() });
            }
        }
        at = end;
    }
    result
}

/// Collapses a short group said twice in a row, as in "I think I think".
fn collapse_groups(mut words: Vec<Word>) -> Vec<Word> {
    for size in [3usize, 2] {
        let mut at = 0;
        while at + size * 2 <= words.len() {
            let (first, second) = (&words[at..at + size], &words[at + size..at + size * 2]);
            let same = first.iter().zip(second).all(|(a, b)| a.core.to_lowercase() == b.core.to_lowercase());
            let joined = first.iter().chain(&second[..size - 1]).all(|word| word.trail.is_empty())
                && first.iter().skip(1).chain(second).all(|word| word.lead.is_empty());
            let keys: Vec<String> = first.iter().map(|word| fold(&word.core)).collect();
            let plain = first.iter().all(|word| is_plain_word(&word.core));
            // "de la de la" is a stumble, while "de amor de amor" and "por favor por favor" are meant.
            let tripped = FALSE_STARTS.contains(&keys[0].as_str()) || keys.iter().all(|key| TRIPPED_OVER.contains(&key.as_str()));
            let meant = keys.iter().any(|key| JOINING_WORDS.contains(&key.as_str()) || KEEP_REPEATED.contains(&key.as_str()));
            let spelled_letter = size == 2 && first[1].core.chars().count() < 2;
            // "paso a paso a paso" repeats on purpose, and it shows in the word before the group.
            let pattern_continues = at > 0 && fold(&words[at - 1].core) == keys[1];
            if same && joined && plain && tripped && !meant && !spelled_letter && !pattern_continues {
                let last_trail = second[size - 1].trail.clone();
                words.drain(at + size..at + size * 2);
                words[at + size - 1].trail = last_trail;
            } else {
                at += 1;
            }
        }
    }
    words
}

/// Drops a cut-off start of a word, as in "qui- quiero".
fn drop_cut_off_words(words: Vec<Word>) -> Vec<Word> {
    let mut result: Vec<Word> = Vec::with_capacity(words.len());
    let mut capitalize_next = false;
    for (index, word) in words.iter().enumerate() {
        let cut_off = word.trail == "-"
            && is_plain_word(&word.core)
            && words.get(index + 1).is_some_and(|next| {
                next.lead.is_empty() && next.core.len() > word.core.len() && fold(&next.core).starts_with(&fold(&word.core))
            });
        if cut_off {
            capitalize_next = starts_upper(&word.core);
            continue;
        }
        let mut word = word.clone();
        if capitalize_next {
            word.core = capitalized(&word.core);
            capitalize_next = false;
        }
        result.push(word);
    }
    result
}

fn collapse_stutters(pieces: Vec<Piece>) -> Vec<Piece> {
    let mut result = Vec::with_capacity(pieces.len());
    let mut stretch: Vec<Word> = Vec::new();
    let finish = |stretch: &mut Vec<Word>, result: &mut Vec<Piece>| {
        let words = collapse_groups(collapse_runs(drop_cut_off_words(std::mem::take(stretch))));
        result.extend(words.into_iter().map(Piece::Word));
    };
    for piece in pieces {
        match piece {
            Piece::Word(word) => stretch.push(word),
            Piece::Break(_) => {
                finish(&mut stretch, &mut result);
                result.push(piece);
            }
        }
    }
    finish(&mut stretch, &mut result);
    result
}

fn render(pieces: &[Piece]) -> String {
    let mut text = String::new();
    let mut after_break = false;
    for piece in pieces {
        match piece {
            Piece::Break(line_break) => {
                text.push_str(line_break);
                after_break = true;
            }
            Piece::Word(word) => {
                if !text.is_empty() && !after_break {
                    text.push(' ');
                }
                text.push_str(&word.lead);
                if after_break {
                    text.push_str(&capitalized(&word.core));
                } else {
                    text.push_str(&word.core);
                }
                text.push_str(&word.trail);
                after_break = false;
            }
        }
    }
    text
}

/// Parakeet can write "Hola,¿cómo" with no space, which would hide both words from the rules.
fn with_space_before_opening_marks(text: &str) -> String {
    let mut spaced = String::with_capacity(text.len() + 4);
    let mut previous = ' ';
    for c in text.chars() {
        if matches!(c, '¿' | '¡') && !previous.is_whitespace() && !matches!(previous, '¿' | '¡' | '(' | '"' | '«') {
            spaced.push(' ');
        }
        spaced.push(c);
        previous = c;
    }
    spaced
}

/// Only Spanish and English have rules so far. Detected or unknown languages use them too,
/// because Parakeet doesn't say which language it heard.
fn has_rules(language: Option<&str>) -> bool {
    language.is_none_or(|code| matches!(code, "es" | "en"))
}

/// Cleans one dictation. `language` is the language chosen for Whisper, if any.
pub fn tidy(text: &str, language: Option<&str>) -> String {
    if !has_rules(language) {
        return text.to_string();
    }
    let spaced = with_space_before_opening_marks(text);
    let words: Vec<Word> = spaced.split_whitespace().map(split_word).collect();
    render(&crate::lists::make_lists(collapse_stutters(remove_fillers(apply_commands(words)))))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(cases: &[(&str, &str)]) {
        for (said, expected) in cases {
            assert_eq!(&tidy(said, None), expected, "for: {said}");
        }
    }

    fn unchanged(cases: &[&str]) {
        for said in cases {
            assert_eq!(&tidy(said, None), said);
        }
    }

    #[test]
    fn filler_sounds_are_removed_in_english() {
        check(&[
            ("Um, I think we should, uh, leave early.", "I think we should leave early."),
            ("I um think so.", "I think so."),
            ("Uhh, okay.", "Okay."),
            ("Hmm, let me see.", "Let me see."),
            ("We can leave, um.", "We can leave."),
            ("Done. Um, next one.", "Done. Next one."),
            ("Erm, yes.", "Yes."),
        ]);
    }

    #[test]
    fn filler_sounds_are_removed_in_spanish() {
        check(&[
            ("Eh, creo que, em, deberíamos salir temprano.", "Creo que deberíamos salir temprano."),
            ("Mmm, no sé.", "No sé."),
            ("Ehh, ¿cómo estás?", "¿Cómo estás?"),
            ("Eh siendo una prueba una vez más.", "Siendo una prueba una vez más."),
            ("Eh,¿cómo estás?", "¿Cómo estás?"),
            ("Hola, eh, ¿cómo estás?", "Hola, ¿cómo estás?"),
        ]);
    }

    #[test]
    fn este_is_removed_when_it_is_a_hesitation() {
        check(&[
            ("Este, quería preguntarte algo.", "Quería preguntarte algo."),
            ("Este... no sé qué decirte.", "No sé qué decirte."),
            ("Fui a la, este, tienda.", "Fui a la tienda."),
            // The model wrote the pause as periods, as it did with the chime test clips.
            ("Fui a la. Este. Tienda de la esquina.", "Fui a la tienda de la esquina."),
            ("Pues este. No sé qué decirte.", "Pues no sé qué decirte."),
            ("Bueno este. Ahora vamos a ver.", "Bueno ahora vamos a ver."),
            ("Es una. Este. Casa de la ONU.", "Es una casa de la ONU."),
            ("Fui a la, este... tienda.", "Fui a la tienda."),
            ("Fui a la, este este, tienda.", "Fui a la tienda."),
            ("Pues este, no sé.", "Pues no sé."),
            ("Hola. Este, te llamo mañana.", "Hola. Te llamo mañana."),
            ("Esteee, mañana te llamo.", "Mañana te llamo."),
            ("Y esteee pues ya.", "Y pues ya."),
            ("Este, ¿cómo estás?", "¿Cómo estás?"),
            ("Eh, este, quería decirte algo.", "Quería decirte algo."),
            ("Hola Juan, este, nueva línea, gracias.", "Hola Juan,\nGracias."),
            // How Parakeet and Whisper Turbo wrote the test dictations.
            ("Este. Quería preguntarte algo.", "Quería preguntarte algo."),
            ("Hola Juan. Este. Te llamo mañana.", "Hola Juan. Te llamo mañana."),
            ("Fui a la... este tienda de la esquina.", "Fui a la tienda de la esquina."),
            ("Fui a la... este... tienda de la esquina.", "Fui a la tienda de la esquina."),
            ("Pues este... no sé qué decirte.", "Pues no sé qué decirte."),
            ("Hola Juan. Este, te llamo mañana.", "Hola Juan. Te llamo mañana."),
            ("Compré una este bolsa.", "Compré una bolsa."),
            // The owner's own dictations, written with no pause after "este".
            ("Este quería preguntarte algo.", "Quería preguntarte algo."),
            (
                "bueno estamos probando otra una vez de nuevo otra vez la aplicación de mimi este ahora vamos a ver una segunda prueba",
                "bueno estamos probando otra una vez de nuevo otra vez la aplicación de mimi ahora vamos a ver una segunda prueba",
            ),
            ("Este yo creo que sí.", "Yo creo que sí."),
            ("Este el otro día fui al cine.", "El otro día fui al cine."),
            ("Pues este vamos a ver.", "Pues vamos a ver."),
            ("Este eh no sé.", "No sé."),
            ("Hola. Este necesito un favor.", "Hola. Necesito un favor."),
            (
                "Bueno, estamos probando la aplicación de Mimi, este ahora vamos a ver.",
                "Bueno, estamos probando la aplicación de Mimi, ahora vamos a ver.",
            ),
        ]);
    }

    #[test]
    fn este_meaning_this_stays() {
        unchanged(&[
            "Este libro es bueno.",
            "Este es mi libro.",
            "Me gusta este.",
            "Quiero este, no ese.",
            "Me quedo con este, gracias.",
            "Ese, este y aquel.",
            "Quiero ese, este y aquel.",
            "Compré dos: este, y ese.",
            "¿Este?",
            "¿Este, o ese?",
            "Este año fue bueno.",
            "Vivo al este, cerca del río.",
            "Este de aquí.",
            "Estela llegó.",
            "Estés donde estés.",
            "¿Cuál quieres? Este. Gracias.",
            "Quiero este. No ese.",
            "Este.",
            "¿Este? Quería preguntarte algo.",
            "Vivo en el este.",
            "Vamos al... este de la ciudad.",
            "Bueno... este libro me gusta.",
            "Hablamos de... este tema.",
            "Quiero este ahora.",
            "Dame este el viernes.",
            "Este no me gusta.",
            "Este ya funciona.",
            "Este se rompió.",
            "Este también me gusta.",
            "Este día fue largo.",
            "Este mes vamos a viajar.",
            "Este fin de semana voy a la playa.",
        ]);
    }

    #[test]
    fn a_dictation_of_only_fillers_leaves_nothing() {
        check(&[("Um.", ""), ("Eh, um, hmm.", "")]);
    }

    #[test]
    fn real_words_that_look_like_fillers_stay() {
        unchanged(&[
            "Está bien, ¿eh?",
            "¡Eh, tú!",
            "Use an em dash here.",
            "Este libro es bueno.",
            "Ah, ya entiendo.",
            "I like this, you know.",
            "O sea que pues sí.",
            "The umbrella and the humming ember.",
            "Mhm.",
        ]);
    }

    #[test]
    fn stutters_collapse() {
        check(&[
            ("I I I think so.", "I think so."),
            ("I, I, I think so.", "I think so."),
            ("The the plan works.", "The plan works."),
            ("Yo yo yo creo que el el plan es bueno.", "Yo creo que el plan es bueno."),
            ("Yo, yo, yo creo.", "Yo creo."),
            ("Quiero quiero ir.", "Quiero ir."),
            ("Don't don't do it.", "Don't do it."),
            ("Dime dime dime qué pasó.", "Dime qué pasó."),
            ("I think I think we can.", "I think we can."),
            ("Estoy probando estoy probando mucho.", "Estoy probando mucho."),
            ("Vamos a la a la casa.", "Vamos a la casa."),
            ("I want to I want to go.", "I want to go."),
            ("Qui- quiero ir.", "Quiero ir."),
            ("Wa- want it.", "Want it."),
            ("I think that that that works.", "I think that that works."),
            ("Es que él el el el el plan.", "Es que el plan."),
            ("El plan es muy muy muy muy muy muy muy muy muy muy muy bueno.", "El plan es muy muy bueno."),
            ("It is very very very very good.", "It is very very good."),
            ("Es muy, muy, muy, muy bueno.", "Es muy, muy bueno."),
            ("No no no no no.", "No no."),
            ("Dime dime dime dime dime.", "Dime."),
        ]);
    }

    #[test]
    fn real_repetition_stays() {
        unchanged(&[
            "It is very very good.",
            "Es muy muy bueno.",
            "Es muy, muy bueno.",
            "No no no.",
            "No, no, eso no.",
            "Sí, sí, claro.",
            "I think that that works.",
            "He had had enough.",
            "Bye bye.",
            "Ja ja ja.",
            "Gracias, gracias.",
            "Thank you thank you.",
            "Por favor por favor.",
            "Little by little, day by day.",
            "On and on and on.",
            "More and more and more.",
            "Paso a paso a paso.",
            "Poco a poco.",
            "My number is 55 55 12.",
            "Cinco cinco dos.",
            "Cinco cinco cinco cinco uno dos.",
            "Ja ja ja ja ja ja.",
            "Muy muy muy bueno.",
            "Fine. Fine.",
            "A él el plan le gusta.",
            "Sí si puedes.",
            "New York New York.",
            "Perdón perdón, solo solo perdón.",
            "Fruto de amor de amor.",
            "Mira mira.",
            "Juan Juan Pérez llamó.",
            "A ver a ver.",
            "Of course of course.",
        ]);
    }

    #[test]
    fn line_break_commands_work_in_both_languages() {
        check(&[
            ("I think we should go. New line. See you tomorrow.", "I think we should go.\nSee you tomorrow."),
            ("Hola Juan. Nueva línea. Gracias por tu mensaje.", "Hola Juan.\nGracias por tu mensaje."),
            ("Hola Juan. Salto de línea. Gracias.", "Hola Juan.\nGracias."),
            ("The topic is done. New paragraph. The next one.", "The topic is done.\n\nThe next one."),
            ("Terminé. Nuevo párrafo. Ahora sigo.", "Terminé.\n\nAhora sigo."),
            ("Llegamos tarde. Punto y aparte. Mañana seguimos.", "Llegamos tarde.\n\nMañana seguimos."),
            ("Llegamos tarde, punto y aparte. Mañana seguimos.", "Llegamos tarde.\n\nMañana seguimos."),
            ("Terminamos, punto y aparte, seguimos mañana.", "Terminamos.\n\nSeguimos mañana."),
            ("Dear John, new line. Thanks for your message.", "Dear John,\nThanks for your message."),
            ("Hola Juan. Nueva línea, gracias.", "Hola Juan.\nGracias."),
            ("Hola Juan, Nueva línea,¿Cómo estás?", "Hola Juan,\n¿Cómo estás?"),
            ("Hola Juan, nueva línea Gracias por tu mensaje", "Hola Juan,\nGracias por tu mensaje"),
            ("Terminé. Nuevo párrafo Ahora sigo.", "Terminé.\n\nAhora sigo."),
            ("We are done, new line Thanks for everything.", "We are done.\nThanks for everything."),
            ("Hola Juan, Nueva Línea,¿cómo estás? Nueva línea, saludos", "Hola Juan,\n¿Cómo estás?\nSaludos"),
            ("¿Cómo estás? Nueva línea. Bien.", "¿Cómo estás?\nBien."),
            ("How are you? New paragraph. Fine.", "How are you?\n\nFine."),
            ("Ya llegué! Nueva línea, adiós.", "Ya llegué!\nAdiós."),
            ("Hola Juan, nueva línea, gracias por tu mensaje.", "Hola Juan,\nGracias por tu mensaje."),
            ("Hola, Juan, nueva línea, gracias por tu mensaje.", "Hola, Juan,\nGracias por tu mensaje."),
            ("Hola Juan, Nueva Línea, gracias.", "Hola Juan,\nGracias."),
            ("I think we should go, new line, see you tomorrow.", "I think we should go.\nSee you tomorrow."),
            ("The report is ready, new paragraph, please review it today.", "The report is ready.\n\nPlease review it today."),
            ("El informe está listo, nuevo párrafo. Por favor revísalo hoy.", "El informe está listo.\n\nPor favor revísalo hoy."),
            ("Hola, gracias por tu ayuda y tu tiempo, nueva línea, nos vemos pronto y cuídate mucho hoy.", "Hola, gracias por tu ayuda y tu tiempo.\nNos vemos pronto y cuídate mucho hoy."),
            ("Hola. Nueva linea. Adiós.", "Hola.\nAdiós."),
            ("Hola. Newline. Adiós.", "Hola.\nAdiós."),
            ("Hola. New line. New line. Adiós.", "Hola.\n\nAdiós."),
        ]);
    }

    #[test]
    fn commands_at_the_edges_are_kept_for_the_next_dictation() {
        check(&[
            ("New line. Hello.", "\nHello."),
            ("Hello. New line.", "Hello.\n"),
            ("Hello. New line", "Hello.\n"),
            ("New line.", "\n"),
        ]);
    }

    #[test]
    fn the_same_words_in_a_sentence_are_not_commands() {
        unchanged(&[
            "We launched a new line of products this year.",
            "Lanzamos una nueva línea de productos este año.",
            "Please add a new paragraph about the budget.",
            "Por favor agrega un nuevo párrafo sobre el presupuesto.",
            "Dear John New Line Thanks for your message.",
            "Hola Juan Nueva Línea, gracias por tu mensaje.",
            "Tres cosas: nueva línea, nuevo logo y nueva web.",
            "Vendemos camisas, nueva línea, y pantalones.",
            "Camisas, pantalones, nueva línea, zapatos.",
            "Lanzamos logo, nueva línea, nuevo empaque y web.",
            "We have a logo, new line, new packaging and a website.",
            "New Line Cinema made it.",
            "Hola. New Line Cinema made it.",
            "Compramos la nueva línea Nike.",
            "We bought a new line Thanks to you.",
            "¿Nueva línea? Sí.",
            "Is that a new line?",
            "El punto y aparte se usa al final.",
        ]);
    }

    #[test]
    fn rules_work_together() {
        check(&[(
            "Um, hola, hola Juan. Nueva línea. Eh, yo yo creo que, em, sí.",
            "Hola, hola Juan.\nYo creo que sí.",
        )]);
    }

    #[test]
    fn other_languages_are_left_alone() {
        let japanese = "はい はい、えーと 今日は晴れです。";
        assert_eq!(tidy(japanese, Some("ja")), japanese);
        assert_eq!(tidy(japanese, None), japanese);
        assert_eq!(tidy("Eu tenho um carro.", Some("pt")), "Eu tenho um carro.");
        assert_eq!(tidy("I um think.", Some("en")), "I think.");
    }

    #[test]
    fn empty_and_odd_text_is_safe() {
        check(&[("", ""), ("   ", ""), ("...", "..."), ("- -", "- -"), ("¿?", "¿?")]);
    }

    #[test]
    fn extra_spaces_are_the_only_change_to_clean_text() {
        assert_eq!(tidy("Hola,   ¿cómo estás?  Bien.", None), "Hola, ¿cómo estás? Bien.");
        assert_eq!(tidy("Hola,¿cómo estás? (¿Bien?) ¡¿Qué?!", None), "Hola, ¿cómo estás? (¿Bien?) ¡¿Qué?!");
    }
}
