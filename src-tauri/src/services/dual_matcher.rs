use crate::models::history::MayhemAugmentItem;

pub fn strip_html_and_placeholders(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut in_tag = false;
    let mut in_var = false;
    let mut in_curly = false;

    for c in raw.chars() {
        if c == '<' {
            in_tag = true;
            continue;
        }
        if c == '>' && in_tag {
            in_tag = false;
            out.push(' ');
            continue;
        }
        if c == '@' {
            in_var = !in_var;
            continue;
        }
        if c == '{' {
            in_curly = true;
            continue;
        }
        if c == '}' && in_curly {
            in_curly = false;
            continue;
        }
        if !in_tag && !in_var && !in_curly {
            if c.is_alphanumeric() || c.is_whitespace() {
                out.push(c);
            } else {
                out.push(' ');
            }
        }
    }
    out
}

pub fn normalize_token(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .collect::<String>()
        .to_lowercase()
}

pub fn word_levenshtein(w1: &str, w2: &str) -> usize {
    let v1: Vec<char> = w1.chars().collect();
    let v2: Vec<char> = w2.chars().collect();
    let l1 = v1.len();
    let l2 = v2.len();
    if l1 == 0 { return l2; }
    if l2 == 0 { return l1; }
    let mut prev = (0..=l2).collect::<Vec<usize>>();
    let mut curr = vec![0; l2 + 1];

    for i in 0..l1 {
        curr[0] = i + 1;
        for j in 0..l2 {
            let cost = if v1[i] == v2[j] { 0 } else { 1 };
            curr[j + 1] = (curr[j + 1] + 1).min(prev[j + 1] + 1).min(prev[j] + cost);
        }
        prev.copy_from_slice(&curr);
    }
    prev[l2]
}

pub fn word_similarity(w1: &str, w2: &str) -> f64 {
    if w1 == w2 { return 1.0; }
    let max_len = w1.len().max(w2.len());
    if max_len == 0 { return 1.0; }
    let dist = word_levenshtein(w1, w2);
    1.0 - (dist as f64 / max_len as f64)
}

pub struct DualSignalMatcher {
    pub title_weight: f64,
    pub desc_weight: f64,
}

impl Default for DualSignalMatcher {
    fn default() -> Self {
        Self {
            title_weight: 0.65,
            desc_weight: 0.35,
        }
    }
}

impl DualSignalMatcher {
    pub fn score_candidate(
        &self,
        col_lines: &[String],
        col_text: &str,
        col_words: &[String],
        aug: &MayhemAugmentItem,
    ) -> (f64, f64, f64) {
        let aug_title_clean = normalize_token(&aug.name);
        if aug_title_clean.is_empty() {
            return (0.0, 0.0, 0.0);
        }
        let aug_title_words: Vec<String> = aug.name
            .split_whitespace()
            .map(normalize_token)
            .filter(|w| !w.is_empty())
            .collect();

        // 1. Title matching score
        let mut title_score: f64 = 0.0;

        // A. Check exact or very high similarity against individual lines
        for line in col_lines {
            let line_norm = normalize_token(line);
            if line_norm.is_empty() {
                continue;
            }
            if line_norm == aug_title_clean {
                title_score = title_score.max(1.0);
            } else {
                let sim = word_similarity(&line_norm, &aug_title_clean);
                if sim >= 0.80 {
                    title_score = title_score.max(sim);
                }
            }
        }

        // B. Check word sequence matching in col_words
        if !aug_title_words.is_empty() {
            let mut matched_title_words = 0;
            for tw in &aug_title_words {
                if col_words.iter().any(|cw| {
                    cw == tw
                        || (cw.len() >= 4
                            && tw.len() >= 4
                            && word_similarity(cw, tw) >= 0.75)
                }) {
                    matched_title_words += 1;
                }
            }
            let word_ratio = matched_title_words as f64 / aug_title_words.len() as f64;
            if word_ratio >= 0.99 {
                title_score = title_score.max(0.95);
            } else if word_ratio >= 0.5 {
                // Penalize missing words in multi-word titles (e.g. "Infinite Recursion" vs "Recursion")
                title_score = title_score.max(0.55 * word_ratio);
            }
        }

        // C. Check full combined col_text contains title (for split-line titles like "Brass \n Blast")
        if title_score < 0.90 && col_text.contains(&aug_title_clean) {
            title_score = title_score.max(0.90);
        }

        // 2. Description keyword matching score
        let mut desc_score: f64 = 0.0;
        if let Some(tt) = aug.tooltip.as_deref() {
            let clean_tt = strip_html_and_placeholders(tt);
            let tt_keywords: Vec<String> = clean_tt
                .split_whitespace()
                .map(normalize_token)
                .filter(|w| w.len() >= 4)
                .filter(|w| !matches!(w.as_str(), "your" | "gain" | "deal" | "when" | "this" | "from" | "with" | "after" | "each" | "more" | "than" | "into" | "have"))
                .collect();

            if !tt_keywords.is_empty() {
                let mut matched_desc_words = 0;
                for kw in &tt_keywords {
                    if col_words.iter().any(|cw| cw == kw || (cw.len() >= 5 && kw.len() >= 5 && word_similarity(cw, kw) >= 0.75)) {
                        matched_desc_words += 1;
                    }
                }
                let ratio = matched_desc_words as f64 / tt_keywords.len().min(8) as f64;
                desc_score = ratio.min(1.0);
            }
        }

        if title_score < 0.40 {
            return (0.0, title_score, desc_score);
        }

        let total = (title_score * self.title_weight) + (desc_score * self.desc_weight);
        (total, title_score, desc_score)
    }
}
