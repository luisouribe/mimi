//! Reusable texts inserted by saying a phrase, such as an address or a standard reply.
//!
//! A phrase is replaced wherever it's heard, including inside a sentence: "mándala a mi
//! dirección" becomes "mándala a Calle Luna 5". The owner chose this over requiring pauses,
//! because the speech models don't reliably mark a pause in real speech. Choosing a phrase
//! that isn't said in normal speech is what prevents accidental inserts.

use crate::tidy::{self, Edge, Word};
use serde::Serialize;

const MAX_TEXT_CHARS: usize = 20_000;
/// Languages written without spaces have no word count to check, so this many characters are required.
const MIN_UNSPACED_CHARS: usize = 3;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Snippet {
    pub id: i64,
    pub phrase: String,
    pub text: String,
}

/// A word in a form that ignores capitals, accents and hyphens, because the speech models
/// can write the same word as "email" or "e-mail".
fn word_key(core: &str) -> String {
    tidy::fold(core).replace('-', "")
}

/// The words of a phrase, ignoring punctuation around them.
pub fn phrase_words(phrase: &str) -> Vec<String> {
    phrase
        .split_whitespace()
        .map(|word| word_key(&tidy::split_word(word).core))
        .filter(|word| !word.is_empty())
        .collect()
}

fn is_unspaced_language(word: &str) -> bool {
    word.chars().all(|c| c >= '\u{3000}')
}

/// Why a snippet can't be saved, in words for the person using Mimi.
pub fn problem_with(phrase: &str, text: &str) -> Option<String> {
    let words = phrase_words(phrase);
    if words.is_empty() {
        return Some("Write the phrase you'll say.".to_string());
    }
    if text.trim().is_empty() {
        return Some("Write the text to insert.".to_string());
    }
    if text.chars().count() > MAX_TEXT_CHARS {
        return Some("That text is too long. The limit is 20,000 characters.".to_string());
    }
    let unspaced = words.len() == 1 && words[0].chars().count() >= MIN_UNSPACED_CHARS && is_unspaced_language(&words[0]);
    if words.len() < 2 && !unspaced {
        return Some("Use at least two words, so normal speech doesn't set it off.".to_string());
    }
    if tidy::is_command(&words) {
        return Some("That phrase is already a line break command.".to_string());
    }
    None
}

/// The longest snippet whose phrase starts at this word. Punctuation can only be around the
/// phrase, because "mi, dirección" is two separate thoughts.
fn snippet_at<'a>(words: &[Word], at: usize, phrases: &'a [(Vec<String>, &Snippet)]) -> Option<(usize, &'a Snippet)> {
    phrases.iter().find_map(|(phrase, snippet)| {
        let found = words.get(at..at + phrase.len())?;
        let same = found.iter().zip(phrase).all(|(word, part)| word_key(&word.core) == *part);
        let joined = found.iter().enumerate().all(|(index, word)| {
            (index == 0 || word.lead.is_empty()) && (index + 1 == found.len() || word.trail.is_empty())
        });
        (same && joined).then_some((phrase.len(), *snippet))
    })
}

fn expand_line(line: &str, phrases: &[(Vec<String>, &Snippet)]) -> String {
    let words: Vec<Word> = line.split_whitespace().map(tidy::split_word).collect();
    let mut result = String::with_capacity(line.len());
    let mut changed = false;
    let mut at = 0;
    while at < words.len() {
        if !result.is_empty() {
            result.push(' ');
        }
        if let Some((length, snippet)) = snippet_at(&words, at, phrases) {
            changed = true;
            let starts_sentence = tidy::edge_before(at.checked_sub(1).and_then(|previous| words.get(previous))) == Edge::Hard;
            let ends_line = at + length == words.len();
            let ends_in_letter = snippet.text.chars().last().is_some_and(char::is_alphanumeric);
            let question = words[at + length - 1].trail.contains('?');
            // A phrase alone in its sentence at the end needs no period the model added,
            // which would otherwise stick to a link or an address.
            let plain_edges = words[at].lead.is_empty() && !words[at + length - 1].trail.contains(')');
            let said_alone_at_end = starts_sentence && ends_line && !question && plain_edges;
            if !said_alone_at_end {
                result.push_str(&words[at].lead);
            }
            result.push_str(&snippet.text);
            if !said_alone_at_end && (ends_in_letter || question) {
                result.push_str(&words[at + length - 1].trail);
            }
            at += length;
        } else {
            let word = &words[at];
            result.push_str(&word.lead);
            result.push_str(&word.core);
            result.push_str(&word.trail);
            at += 1;
        }
    }
    let mut result = if changed { result } else { line.to_string() };
    // Japanese and Chinese have no spaces, so the phrase can sit in the middle of a written word.
    for (phrase, snippet) in phrases {
        if phrase.len() == 1 && is_unspaced_language(&phrase[0]) && result.contains(&phrase[0]) {
            result = result.replace(&phrase[0], &snippet.text);
        }
    }
    result
}

/// Replaces every snippet phrase with its text.
pub fn expand(text: &str, snippets: &[Snippet]) -> String {
    if snippets.is_empty() {
        return text.to_string();
    }
    let mut phrases: Vec<(Vec<String>, &Snippet)> =
        snippets.iter().map(|snippet| (phrase_words(&snippet.phrase), snippet)).filter(|(words, _)| !words.is_empty()).collect();
    // Longest first, so "mi dirección de casa" wins over "mi dirección".
    phrases.sort_by_key(|(words, _)| std::cmp::Reverse(words.len()));
    text.split('\n').map(|line| expand_line(line, &phrases)).collect::<Vec<_>>().join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    const ADDRESS: &str = "Calle Luna 5";
    const LINK: &str = "https://example.com/mimi";

    fn saved() -> Vec<Snippet> {
        vec![
            Snippet { id: 1, phrase: "mi dirección".into(), text: ADDRESS.into() },
            Snippet { id: 2, phrase: "My Email".into(), text: "luis@example.com".into() },
            Snippet { id: 3, phrase: "mi enlace".into(), text: LINK.into() },
            Snippet { id: 4, phrase: "mi dirección de trabajo".into(), text: "Oficina 9.".into() },
            Snippet { id: 5, phrase: "respuesta estándar".into(), text: "Gracias por escribir.\nTe contesto pronto.".into() },
        ]
    }

    fn check(cases: &[(&str, &str)]) {
        for (said, expected) in cases {
            assert_eq!(&expand(said, &saved()), expected, "for: {said}");
        }
    }

    #[test]
    fn a_phrase_said_alone_inserts_the_text() {
        check(&[
            ("Mi dirección.", ADDRESS),
            ("mi dirección", ADDRESS),
            ("MI DIRECCION.", ADDRESS),
            ("My email.", "luis@example.com"),
            ("My e-mail.", "luis@example.com"),
            ("Mi enlace.", LINK),
            ("Te mando esto. Mi enlace.", "Te mando esto. https://example.com/mimi"),
            ("Respuesta estandar.", "Gracias por escribir.\nTe contesto pronto."),
        ]);
    }

    /// These four are what the speech model wrote from the owner's own test dictations.
    #[test]
    fn a_phrase_inside_a_real_sentence_inserts_the_text() {
        check(&[
            ("Hola Juan. Manda la mi dirección. Saludos.", "Hola Juan. Manda la Calle Luna 5. Saludos."),
            ("Hola Juancy, puedes mandarla a mi dirección, saludos", "Hola Juancy, puedes mandarla a Calle Luna 5, saludos"),
            ("Hola, sí Puedes mandarla a mi dirección Gracias Juan saludos", "Hola, sí Puedes mandarla a Calle Luna 5 Gracias Juan saludos"),
            (
                "Hola Juan, si puedes mandarlo porfa mi dirección Gracias por el mensaje, saludos",
                "Hola Juan, si puedes mandarlo porfa Calle Luna 5 Gracias por el mensaje, saludos",
            ),
        ]);
    }

    #[test]
    fn punctuation_around_the_phrase_is_kept() {
        check(&[
            ("Hola Juan. Mi dirección. Gracias.", "Hola Juan. Calle Luna 5. Gracias."),
            ("Hola, mi dirección, gracias.", "Hola, Calle Luna 5, gracias."),
            ("Envíalo a mi dirección.", "Envíalo a Calle Luna 5."),
            ("¿Es mi dirección?", "¿Es Calle Luna 5?"),
            ("¿Mi dirección?", "¿Calle Luna 5?"),
            ("Send it to my email, please.", "Send it to luis@example.com, please."),
            ("(mi dirección)", "(Calle Luna 5)"),
        ]);
    }

    #[test]
    fn the_longest_phrase_wins_and_several_can_appear() {
        check(&[
            ("Mi dirección de trabajo.", "Oficina 9."),
            ("Mi dirección y mi enlace.", "Calle Luna 5 y https://example.com/mimi."),
        ]);
    }

    #[test]
    fn a_text_ending_in_punctuation_does_not_double_it() {
        check(&[("Hola. Mi dirección de trabajo. Adiós.", "Hola. Oficina 9. Adiós.")]);
    }

    #[test]
    fn it_works_on_each_line() {
        check(&[("Hola.\nMi enlace.\n\nAdiós.", "Hola.\nhttps://example.com/mimi\n\nAdiós.")]);
    }

    #[test]
    fn japanese_phrases_work_inside_a_written_word() {
        let saved = vec![Snippet { id: 1, phrase: "私の住所".into(), text: "東京都".into() }];
        assert_eq!(expand("荷物は私の住所に送ってください。", &saved), "荷物は東京都に送ってください。");
        assert_eq!(expand("荷物を送ってください。", &saved), "荷物を送ってください。");
    }

    #[test]
    fn only_the_whole_phrase_counts() {
        for said in [
            "Mi nueva dirección es Calle Luna.",
            "Cambié de dirección ayer.",
            "Mi, dirección.",
            "Mis direcciones.",
            "Mi dirección-postal.",
            "My emails are late.",
            "My new email works.",
            "Respuesta estándares.",
        ] {
            assert_eq!(&expand(said, &saved()), said);
        }
    }

    #[test]
    fn text_without_a_phrase_is_exactly_the_same() {
        let text = "Hola,   sin cambios.\n\n  Otra línea.  ";
        assert_eq!(expand(text, &saved()), text);
        assert_eq!(expand(text, &[]), text);
    }

    #[test]
    fn phrases_that_are_too_risky_are_refused() {
        assert!(problem_with("", "texto").is_some());
        assert!(problem_with("   ", "texto").is_some());
        assert!(problem_with("mi dirección", "   ").is_some());
        assert!(problem_with("dirección", "texto").is_some());
        assert!(problem_with("mañana", "texto").is_some());
        assert!(problem_with("nueva línea", "texto").is_some());
        assert!(problem_with("New Paragraph.", "texto").is_some());
        assert!(problem_with("punto y aparte", "texto").is_some());
        assert!(problem_with("mi dirección", &"x".repeat(20_001)).is_some());
    }

    #[test]
    fn good_phrases_are_accepted() {
        assert_eq!(problem_with("mi dirección", "Calle Luna 5"), None);
        assert_eq!(problem_with("my standard reply", "Thanks!"), None);
        assert_eq!(problem_with("私の住所", "東京都"), None);
    }
}
