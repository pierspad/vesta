use std::collections::HashSet;

use crate::SubtitleLine;

/// Strip markup, SDH sound effect tags, HTML tags, speaker prefixes and normalize contractions.
pub fn strip_markup_and_sdh(text: &str) -> String {
    // 1. Replace common HTML entities
    let unescaped = text
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">");

    // 2. Strip bracketed / parenthetical / HTML content: <...>, [...], (...)
    let mut stripped = String::with_capacity(unescaped.len());
    let mut html_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut paren_depth = 0usize;

    for ch in unescaped.chars() {
        match ch {
            '<' => html_depth += 1,
            '>' => html_depth = html_depth.saturating_sub(1),
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '(' => paren_depth += 1,
            ')' => paren_depth = paren_depth.saturating_sub(1),
            c if html_depth == 0 && bracket_depth == 0 && paren_depth == 0 => {
                stripped.push(c);
            }
            _ => {}
        }
    }

    // 3. Normalize unicode apostrophes/quotes
    let normalized_quotes: String = stripped
        .chars()
        .map(|c| match c {
            '’' | '‘' | '`' | '´' => '\'',
            _ => c,
        })
        .collect();

    // 4. Tokenize and normalize contractions and alphanumeric words
    let mut words = Vec::new();
    for raw_line in normalized_quotes.lines() {
        let mut line = raw_line.trim();

        // Strip leading speaker dashes or labels like "- Sue: " or "MAN: "
        if let Some(rest) = line.strip_prefix('-') {
            line = rest.trim_start();
        }
        if let Some(colon_pos) = line.find(':') {
            // If the text before colon is short and all uppercase or titlecase (e.g. "JOHN:", "Sue:"), strip it
            let prefix = &line[..colon_pos];
            if prefix.len() < 25
                && prefix
                    .chars()
                    .all(|c| c.is_alphabetic() || c.is_whitespace())
            {
                line = line[colon_pos + 1..].trim_start();
            }
        }

        for raw_token in line.split_whitespace() {
            let lower = raw_token.to_lowercase();
            let token = normalize_contraction(&lower);
            // Keep alphanumeric chars only
            let cleaned: String = token
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '\'')
                .collect();
            let clean_no_apostrophe = cleaned.replace('\'', "");
            if clean_no_apostrophe.len() >= 2 {
                words.push(clean_no_apostrophe);
            }
        }
    }

    words.join(" ")
}

fn normalize_contraction(token: &str) -> &str {
    match token.trim_matches(|c: char| !c.is_alphanumeric()) {
        "don't" | "dont" => "dont",
        "can't" | "cant" => "cant",
        "won't" | "wont" => "wont",
        "i'm" | "im" => "im",
        "it's" | "its" => "its",
        "you're" | "youre" => "youre",
        "we're" | "were" => "were",
        "they're" | "theyre" => "theyre",
        "i'll" | "ill" => "ill",
        "you'll" | "youll" => "youll",
        "he'll" | "hell" => "hell",
        "she'll" | "shell" => "shell",
        "we'll" | "well" => "well",
        "they'll" | "theyll" => "theyll",
        "i've" | "ive" => "ive",
        "you've" | "youve" => "youve",
        "we've" | "weve" => "weve",
        "they've" | "theyve" => "theyve",
        "i'd" | "id" => "id",
        "he'd" | "hed" => "hed",
        "she'd" | "shed" => "shed",
        "we'd" | "wed" => "wed",
        "they'd" | "theyd" => "theyd",
        "ain't" | "aint" => "aint",
        "let's" | "lets" => "lets",
        "who's" | "whos" => "whos",
        "what's" | "whats" => "whats",
        "where's" | "wheres" => "wheres",
        "there's" | "theres" => "theres",
        "that's" | "thats" => "thats",
        "didn't" | "didnt" => "didnt",
        "doesn't" | "doesnt" => "doesnt",
        "haven't" | "havent" => "havent",
        "hasn't" | "hasnt" => "hasnt",
        "wouldn't" | "wouldnt" => "wouldnt",
        "couldn't" | "couldnt" => "couldnt",
        "shouldn't" | "shouldnt" => "shouldnt",
        other => other,
    }
}

#[derive(Debug, Clone)]
pub struct CleanText {
    pub raw: String,
    pub clean: String,
    pub tokens: Vec<String>,
    pub token_set: HashSet<String>,
}

impl CleanText {
    pub fn new(text: &str) -> Self {
        let clean = strip_markup_and_sdh(text);
        let tokens: Vec<String> = clean.split_whitespace().map(String::from).collect();
        let token_set: HashSet<String> = tokens.iter().cloned().collect();
        Self {
            raw: text.to_string(),
            clean,
            tokens,
            token_set,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    /// F1 harmonic mean between token precision and recall
    pub fn token_f1(&self, other: &Self) -> f64 {
        if self.tokens.is_empty() || other.tokens.is_empty() {
            return 0.0;
        }
        let overlap = self
            .tokens
            .iter()
            .filter(|token| other.token_set.contains(*token))
            .count() as f64;
        if overlap == 0.0 {
            return 0.0;
        }
        let prec = overlap / other.tokens.len() as f64;
        let rec = overlap / self.tokens.len() as f64;
        (2.0 * prec * rec) / (prec + rec)
    }

    /// Token containment: overlap divided by min(len1, len2).
    /// Highly effective when a transcribed segment is a sub-phrase or covers multiple lines.
    pub fn token_containment(&self, other: &Self) -> f64 {
        if self.tokens.is_empty() || other.tokens.is_empty() {
            return 0.0;
        }
        let overlap = self
            .tokens
            .iter()
            .filter(|token| other.token_set.contains(*token))
            .count() as f64;
        let min_len = self.tokens.len().min(other.tokens.len()) as f64;
        overlap / min_len
    }

    /// Combined similarity blending F1, containment, and character edit similarity
    pub fn similarity(&self, other: &Self) -> f64 {
        let f1 = self.token_f1(other);
        let containment = self.token_containment(other);
        // If high containment, blend it with F1 to reward matching sub-phrases
        let token_score = if containment > 0.8 && f1 > 0.35 {
            (f1 * 0.6) + (containment * 0.4)
        } else {
            f1
        };

        // Character similarity on clean strings
        let c1: Vec<char> = self.clean.chars().collect();
        let c2: Vec<char> = other.clean.chars().collect();
        let char_score = char_similarity(&c1, &c2);

        (token_score * 0.75) + (char_score * 0.25)
    }
}

fn char_similarity(left: &[char], right: &[char]) -> f64 {
    if left.is_empty() || right.is_empty() {
        return 0.0;
    }
    let max_len = left.len().max(right.len()) as f64;
    let dist = levenshtein(left, right) as f64;
    (1.0 - (dist / max_len)).max(0.0)
}

fn levenshtein(a: &[char], b: &[char]) -> usize {
    if a == b {
        return 0;
    }
    let (short, long) = if a.len() <= b.len() { (a, b) } else { (b, a) };
    let mut prev: Vec<usize> = (0..=short.len()).collect();
    let mut curr = vec![0usize; short.len() + 1];

    for (i, &ch_long) in long.iter().enumerate() {
        curr[0] = i + 1;
        for (j, &ch_short) in short.iter().enumerate() {
            let cost = usize::from(ch_long != ch_short);
            curr[j + 1] = (prev[j + 1] + 1).min(curr[j] + 1).min(prev[j] + cost);
        }
        prev.copy_from_slice(&curr);
    }
    prev[short.len()]
}

/// Detect language from subtitle text heuristics (scripts and distinctive stop words).
pub fn detect_subtitle_language(subtitles: &[SubtitleLine]) -> Option<String> {
    if subtitles.is_empty() {
        return None;
    }

    let mut sample_text = String::with_capacity(8192);
    let sample_limit = subtitles.len().min(80);
    for sub in subtitles.iter().take(sample_limit) {
        sample_text.push(' ');
        sample_text.push_str(&sub.text);
    }

    // 1. Script checks
    let mut japanese_chars = 0usize;
    let mut chinese_chars = 0usize;
    let mut korean_chars = 0usize;
    let mut cyrillic_chars = 0usize;
    let mut arabic_chars = 0usize;
    let mut greek_chars = 0usize;
    let mut hebrew_chars = 0usize;
    let mut thai_chars = 0usize;

    for ch in sample_text.chars() {
        match ch as u32 {
            0x3040..=0x30FF => japanese_chars += 1,
            0x4E00..=0x9FFF => chinese_chars += 1,
            0xAC00..=0xD7AF | 0x1100..=0x11FF => korean_chars += 1,
            0x0400..=0x04FF => cyrillic_chars += 1,
            0x0600..=0x06FF => arabic_chars += 1,
            0x0370..=0x03FF => greek_chars += 1,
            0x0590..=0x05FF => hebrew_chars += 1,
            0x0E00..=0x0E7F => thai_chars += 1,
            _ => {}
        }
    }

    if japanese_chars >= 5 {
        return Some("ja".into());
    }
    if chinese_chars >= 10 {
        return Some("zh".into());
    }
    if korean_chars >= 5 {
        return Some("ko".into());
    }
    if cyrillic_chars >= 10 {
        return Some("ru".into());
    }
    if arabic_chars >= 10 {
        return Some("ar".into());
    }
    if greek_chars >= 10 {
        return Some("el".into());
    }
    if hebrew_chars >= 10 {
        return Some("he".into());
    }
    if thai_chars >= 10 {
        return Some("th".into());
    }

    // 2. Latin stop word frequencies
    let clean_sample = strip_markup_and_sdh(&sample_text);
    let words: Vec<&str> = clean_sample.split_whitespace().collect();

    let english_markers = [
        "the", "and", "you", "that", "was", "for", "with", "have", "this", "what", "are", "from",
        "not", "here", "they", "there", "about",
    ];
    let italian_markers = [
        "che", "non", "sono", "una", "per", "con", "questo", "della", "delle", "dello", "tutto",
        "anche", "cosa", "bene", "perche", "grazie",
    ];
    let spanish_markers = [
        "que", "para", "como", "este", "pero", "bien", "todo", "esta", "estoy", "nada", "tiene",
        "gracias", "entonces", "cuando", "donde",
    ];
    let french_markers = [
        "dans", "avec", "pour", "cette", "mais", "tout", "faire", "nous", "vous", "elle", "sont",
        "comme", "bien", "merci",
    ];
    let german_markers = [
        "nicht", "eine", "oder", "auch", "wenn", "nach", "haben", "dieser", "einem", "einer",
        "danke", "immer", "jetzt",
    ];
    let portuguese_markers = [
        "para", "como", "mais", "isso", "esse", "esta", "muito", "estou", "quando", "obrigado",
        "voce", "mesmo",
    ];

    let mut scores = [
        ("en", count_matches(&words, &english_markers)),
        ("it", count_matches(&words, &italian_markers)),
        ("es", count_matches(&words, &spanish_markers)),
        ("fr", count_matches(&words, &french_markers)),
        ("de", count_matches(&words, &german_markers)),
        ("pt", count_matches(&words, &portuguese_markers)),
    ];

    scores.sort_by_key(|a| std::cmp::Reverse(a.1));

    if scores[0].1 >= 4 && scores[0].1 > scores[1].1 {
        Some(scores[0].0.to_string())
    } else {
        None
    }
}

fn count_matches(words: &[&str], markers: &[&str]) -> usize {
    words.iter().filter(|w| markers.contains(w)).count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_markup_and_sdh() {
        let input = "<i>[Radio playing]</i><br>- Sue: Well, here we are. I turn down here at the next block.";
        let cleaned = strip_markup_and_sdh(input);
        assert_eq!(cleaned, "well here we are turn down here at the next block");
    }

    #[test]
    fn test_contraction_normalization() {
        let input = "I don't know, it's not what I'll do, but we're ready.";
        let cleaned = strip_markup_and_sdh(input);
        assert_eq!(cleaned, "dont know its not what ill do but were ready");
    }

    #[test]
    fn test_detect_subtitle_language() {
        let en_subs = vec![
            SubtitleLine {
                id: 1,
                start_ms: 1000,
                text: "Well, here we are. I turn down here at the next block.".into(),
            },
            SubtitleLine {
                id: 2,
                start_ms: 4000,
                text: "Thanks, mister. I'll get off there and see what is with that.".into(),
            },
        ];
        assert_eq!(detect_subtitle_language(&en_subs), Some("en".into()));

        let it_subs = vec![
            SubtitleLine {
                id: 1,
                start_ms: 1000,
                text: "Beh, eccoci qui. Svolta qui alla prossima fermata.".into(),
            },
            SubtitleLine {
                id: 2,
                start_ms: 4000,
                text: "Grazie, signore. Scendo lì che non sono per niente stanco.".into(),
            },
        ];
        assert_eq!(detect_subtitle_language(&it_subs), Some("it".into()));
    }

    #[test]
    fn test_similarity_containment() {
        let whisper = CleanText::new("I turned down here at the next block");
        let sub = CleanText::new(
            "Well, here we are. I turn down here at the next block. Thanks, mister.",
        );
        let sim = whisper.similarity(&sub);
        assert!(
            sim > 0.55,
            "Expected high similarity for subphrase, got {sim}"
        );
    }
}
