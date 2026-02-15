use std::collections::{HashMap, HashSet};
use std::fs;

use regex::Regex;
use serde::Serialize;

use crate::scanner::BiographyFile;
use crate::surname::{build_name_regex, split_name};
use crate::titles::{build_title_regex, STANDALONE_TITLES, TITLE_SUFFIXES};
use crate::types::Person;

// ── Types ────────────────────────────────────────────────────────────

/// Which regex pattern produced this match.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub enum MentionPattern {
    /// 以X為Y — appointment structure
    Appointment,
    /// {title}+{name} — title immediately followed by name
    TitleName,
    /// {name}字{courtesy} — courtesy name introduction
    CourtesyIntro,
    /// 問/謂{name}曰 — speech attribution
    Speech,
}

impl MentionPattern {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Appointment => "以X為",
            Self::TitleName => "官銜+名",
            Self::CourtesyIntro => "X字Y",
            Self::Speech => "問/謂X曰",
        }
    }
}

/// A single in-text mention of a person name.
#[derive(Debug, Clone)]
pub struct InTextMention {
    pub name: String,
    pub surname: String,
    pub given: String,
    pub pattern: MentionPattern,
    /// Short context window around the match
    pub context: String,
    pub source_file: String,
}

/// Aggregated info about a person found via in-text mentions.
#[derive(Debug, Clone, Serialize)]
pub struct InTextPerson {
    pub name: String,
    pub surname: String,
    pub given: String,
    pub mention_count: usize,
    pub mentioned_in: Vec<String>,
    pub pattern_counts: HashMap<String, usize>,
    pub has_own_biography: bool,
    pub sample_contexts: Vec<String>,
}

// ── False positive filtering ─────────────────────────────────────────

/// Strings that look like names (start with a surname char) but are
/// actually title fragments, geographic terms, or fixed expressions.
const BLACKLIST: &[&str] = &[
    // 左/右 as title components
    "左右", "左丞", "右丞", "左曹", "右曹",
    "左僕射", "右僕射",
    "左長史", "右長史",
    "左西屬", "左西掾",
    "左民郎", "左民尚",
    // 黃門 compound
    "黃門侍", "黃門郎",
    // 都官/金部/倉部/祠部 etc. — "部郎" pattern
    "都官郎", "金部郎", "倉部郎", "祠部郎", "殿中郎", "主客郎",
    "度支郎",
    // Geographic + 諸 / multi-state abbreviations
    "江州諸", "荊州諸", "徐州諸", "揚州諸", "豫州諸", "青州諸",
    "荊湘雍", "雍梁南", "徐兗青", "揚徐兗", "雍秦涼",
    // Fixed expressions
    "左氏", "左傳",
];

/// Check if the captured name is a false positive.
fn is_false_positive(name: &str) -> bool {
    // Explicit blacklist
    if BLACKLIST.contains(&name) {
        return true;
    }

    // If the name ends with a title suffix, it's a title chain, not a person.
    // e.g. "左僕射" (ends with 僕射), "黃門侍郎" would be caught differently.
    for suffix in TITLE_SUFFIXES {
        if name.ends_with(suffix) {
            return true;
        }
    }
    for title in STANDALONE_TITLES {
        if name.ends_with(title) {
            return true;
        }
    }

    // If the name ends with a nobility rank AND is 3+ chars, it's a fief title.
    // e.g. "江夏王" (3 chars ending in 王) = King of Jiangxia, not a person.
    // But "王猛" (2 chars starting with 王) is a real person.
    let chars: Vec<char> = name.chars().collect();
    let nobility_suffixes = ['王', '公', '侯'];
    if chars.len() >= 3 {
        if let Some(&last) = chars.last() {
            if nobility_suffixes.contains(&last) {
                return true;
            }
        }
    }

    // If the name ends with 州/郡/縣/國 — geographic, not a person
    let geo_suffixes = ['州', '郡', '縣', '國'];
    if let Some(&last) = chars.last() {
        if geo_suffixes.contains(&last) {
            return true;
        }
    }

    // If the last character is a classical Chinese function word or verb,
    // it was captured from running text, not a real name ending.
    // (Note: 之 is NOT filtered here — it's a common real name suffix
    // in the Six Dynasties period, e.g. 王凝之, 劉穆之)
    let bad_endings: &[char] = &[
        // Prepositions / conjunctions / particles
        '爲', '為', '以', '請', '遣', '使', '令', '命', '率', '及',
        '與', '乃', '則', '即', '既', '又', '且', '而', '所', '於',
        '自', '從', '至', '向', '在', '由', '如', '若', '或', '因',
        '等', '曰', '諸',
        // Common action verbs that follow names and get captured
        '走', '出', '害', '救', '殺', '敗', '收', '攻', '破', '降',
        '反', '叛', '奔', '歸', '入', '克', '圍', '據', '討', '拒',
        '聞', '送', '屯', '還', '還',
        // numbers
        '二', '三', '四', '五', '六', '七', '八', '九', '十',
        '百', '千', '萬',
    ];
    if let Some(&last) = chars.last() {
        if bad_endings.contains(&last) {
            return true;
        }
    }

    false
}

// ── Scanner ──────────────────────────────────────────────────────────

/// Holds compiled regexes for in-text person name extraction.
pub struct InTextScanner {
    /// 以[^為]{0,10}({name})為
    re_appointment: Regex,
    /// ({title})({name})
    re_title_name: Regex,
    /// ({name})字([^\s，。]{1,2})
    re_courtesy: Regex,
    /// [問謂]({name})曰
    re_speech: Regex,
    /// Set of names from persons who have their own biography file
    known_names: HashSet<String>,
}

impl InTextScanner {
    /// Build a new scanner. `known_persons` are the already-parsed biography subjects.
    pub fn new(known_persons: &[Person]) -> Self {
        // Collect extra surnames from known persons
        let extra_surnames = Self::collect_extra_surnames(known_persons);
        let name_re = build_name_regex(&extra_surnames);
        let title_re = build_title_regex();

        // Pattern 1: 以 ... name ... 為
        let re_appointment = Regex::new(&format!(
            "以[^為]{{0,10}}({name_re})為"
        ))
        .expect("appointment regex");

        // Pattern 2: title + name
        let re_title_name = Regex::new(&format!(
            "(?:{title_re})({name_re})"
        ))
        .expect("title_name regex");

        // Pattern 3: name + 字 + courtesy
        let re_courtesy = Regex::new(&format!(
            "({name_re})字([^\\s，。字]{{1,2}})"
        ))
        .expect("courtesy regex");

        // Pattern 4: 問/謂 + name + 曰
        let re_speech = Regex::new(&format!(
            "[問謂]({name_re})曰"
        ))
        .expect("speech regex");

        // Build set of known display names and aliases
        let mut known_names = HashSet::new();
        for p in known_persons {
            known_names.insert(p.display_name());
            for a in &p.aliases {
                if a.chars().count() >= 2 {
                    known_names.insert(a.clone());
                }
            }
        }

        InTextScanner {
            re_appointment,
            re_title_name,
            re_courtesy,
            re_speech,
            known_names,
        }
    }

    fn collect_extra_surnames(persons: &[Person]) -> Vec<String> {
        let mut surnames = HashSet::new();
        for p in persons {
            match &p.kind {
                crate::types::PersonKind::Official { surname, .. }
                | crate::types::PersonKind::Ruler { surname, .. } => {
                    surnames.insert(surname.clone());
                }
                crate::types::PersonKind::Emperor { surname, .. } => {
                    if let Some(s) = surname {
                        surnames.insert(s.clone());
                    }
                }
                _ => {}
            }
        }
        surnames.into_iter().collect()
    }

    /// Scan a single text for person-name mentions.
    pub fn scan_text(&self, content: &str, source_file: &str) -> Vec<InTextMention> {
        let mut mentions = Vec::new();

        // Pattern 1: 以X為
        for caps in self.re_appointment.captures_iter(content) {
            if let Some(m) = caps.get(1) {
                if let Some(mention) =
                    self.make_mention(m.as_str(), MentionPattern::Appointment, content, m.start(), source_file)
                {
                    mentions.push(mention);
                }
            }
        }

        // Pattern 2: title + name
        for caps in self.re_title_name.captures_iter(content) {
            if let Some(m) = caps.get(1) {
                if let Some(mention) =
                    self.make_mention(m.as_str(), MentionPattern::TitleName, content, m.start(), source_file)
                {
                    mentions.push(mention);
                }
            }
        }

        // Pattern 3: name + 字 + courtesy
        for caps in self.re_courtesy.captures_iter(content) {
            if let Some(m) = caps.get(1) {
                if let Some(mention) =
                    self.make_mention(m.as_str(), MentionPattern::CourtesyIntro, content, m.start(), source_file)
                {
                    mentions.push(mention);
                }
            }
        }

        // Pattern 4: speech attribution
        for caps in self.re_speech.captures_iter(content) {
            if let Some(m) = caps.get(1) {
                if let Some(mention) =
                    self.make_mention(m.as_str(), MentionPattern::Speech, content, m.start(), source_file)
                {
                    mentions.push(mention);
                }
            }
        }

        mentions
    }

    /// Validate and construct a mention from a matched name string.
    fn make_mention(
        &self,
        matched: &str,
        pattern: MentionPattern,
        full_text: &str,
        byte_offset: usize,
        source_file: &str,
    ) -> Option<InTextMention> {
        // False positive filter
        if is_false_positive(matched) {
            return None;
        }

        let (surname, given) = split_name(matched)?;

        // Filter: given name must be 1-2 chars
        let given_len = given.chars().count();
        if given_len < 1 || given_len > 2 {
            return None;
        }

        // Extract context window (±20 chars around match)
        let context = extract_context(full_text, byte_offset, 20);

        Some(InTextMention {
            name: matched.to_string(),
            surname,
            given,
            pattern,
            context,
            source_file: source_file.to_string(),
        })
    }

    /// Scan all biography files and return aggregated per-name results.
    pub fn scan_corpus(&self, bio_files: &[BiographyFile]) -> Vec<InTextPerson> {
        // name → (surname, given, mentions-by-file, pattern-counts, contexts)
        let mut agg: HashMap<
            String,
            (
                String,
                String,
                HashSet<String>,
                HashMap<String, usize>,
                Vec<String>,
            ),
        > = HashMap::new();

        for bio in bio_files {
            let content = match fs::read_to_string(&bio.path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let source = bio.path.display().to_string();
            let mentions = self.scan_text(&content, &source);

            for m in mentions {
                let entry = agg.entry(m.name.clone()).or_insert_with(|| {
                    (
                        m.surname.clone(),
                        m.given.clone(),
                        HashSet::new(),
                        HashMap::new(),
                        Vec::new(),
                    )
                });
                entry.2.insert(m.source_file.clone());
                *entry.3.entry(m.pattern.as_str().to_string()).or_insert(0) += 1;
                if entry.4.len() < 3 {
                    entry.4.push(m.context);
                }
            }
        }

        // Convert to sorted vec
        let mut results: Vec<InTextPerson> = agg
            .into_iter()
            .map(|(name, (surname, given, files, patterns, contexts))| {
                let mention_count: usize = patterns.values().sum();
                let has_own_biography = self.known_names.contains(&name);
                let mut mentioned_in: Vec<String> = files.into_iter().collect();
                mentioned_in.sort();

                InTextPerson {
                    name,
                    surname,
                    given,
                    mention_count,
                    mentioned_in,
                    pattern_counts: patterns,
                    has_own_biography,
                    sample_contexts: contexts,
                }
            })
            .collect();

        // Sort by mention count descending
        results.sort_by(|a, b| b.mention_count.cmp(&a.mention_count));
        results
    }
}

/// Extract a context window around a byte offset.
fn extract_context(text: &str, byte_offset: usize, char_radius: usize) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut byte_pos = 0;
    let mut char_idx = 0;
    for (i, ch) in chars.iter().enumerate() {
        if byte_pos >= byte_offset {
            char_idx = i;
            break;
        }
        byte_pos += ch.len_utf8();
    }

    let start = char_idx.saturating_sub(char_radius);
    let end = (char_idx + char_radius).min(chars.len());

    let window: String = chars[start..end].iter().collect();
    let line = window
        .lines()
        .find(|l| !l.is_empty())
        .unwrap_or(&window);

    line.to_string()
}
