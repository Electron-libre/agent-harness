//! Whether a reply is in the conversation language (`user_language` of `.claude/harness.toml`): the test of rule
//! `user-language` (BRE-10, `specs/rule-breaches.md`), shared by `rule-breaches` and the `language-hint` hook
//! (`specs/language-hint.md`) so the measure and the hook agree on what a breach is.

/// Frequent words of one language only (`a`, `on`, `or`, `plus` are both French and English).
const FRENCH_WORDS: &[&str] = &[
    "le", "la", "les", "un", "une", "des", "du", "de", "et", "est", "que", "qui", "dans", "pour",
    "pas", "sur", "avec", "ce", "cette", "il", "elle", "je", "tu", "nous", "vous", "au", "aux",
    "ou", "mais", "par", "sont", "fait", "sans", "comme", "se", "ne", "en", "donc", "puis",
];
const ENGLISH_WORDS: &[&str] = &[
    "the", "an", "and", "is", "are", "that", "which", "in", "for", "not", "with", "this", "it",
    "i", "you", "we", "of", "to", "be", "by", "was", "but", "as", "at", "from", "have", "has",
    "so", "then", "will", "can",
];

/// The fewest words a text needs for its language to be judged.
const MIN_WORDS: usize = 20;

/// A conversation language: its stop words, then those of the other language.
#[derive(Clone, Copy)]
pub(super) struct Language {
    own: &'static [&'static str],
    other: &'static [&'static str],
}

impl Language {
    /// The language named by `user_language`; `None` for a language with no list.
    pub(super) fn named(name: &str) -> Option<Language> {
        match name {
            "French" => Some(Language {
                own: FRENCH_WORDS,
                other: ENGLISH_WORDS,
            }),
            "English" => Some(Language {
                own: ENGLISH_WORDS,
                other: FRENCH_WORDS,
            }),
            _ => None,
        }
    }

    /// Whether `text` holds more stop words of the other language than of this one; `None` when it holds fewer
    /// than `MIN_WORDS` words of prose, too few to judge.
    pub(super) fn breached_by(self, text: &str) -> Option<bool> {
        let words = prose_words(text);
        if words.len() < MIN_WORDS {
            return None;
        }
        let count = |list: &[&str]| words.iter().filter(|w| list.contains(&w.as_str())).count();
        Some(count(self.other) > count(self.own))
    }
}

/// The lower-cased words of a text once fenced code blocks (the fence rule of QUA-12) and inline code are removed.
fn prose_words(text: &str) -> Vec<String> {
    let mut prose = String::new();
    for (line, _) in super::fenced(text.lines()).filter(|&(_, fenced)| !fenced) {
        prose.extend(line.split('`').step_by(2).flat_map(|part| [part, " "]));
        prose.push('\n');
    }
    prose
        .split(|c: char| !c.is_alphabetic())
        .filter(|w| !w.is_empty())
        .map(str::to_lowercase)
        .collect()
}
