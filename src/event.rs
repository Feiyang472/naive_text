//! Event extraction from running text.
//!
//! Extracts structured events (appointments, battles, deaths, transfers)
//! with associated time references and place names from the corpus.

use std::collections::HashMap;
use std::fs;

use regex::Regex;
use serde::Serialize;

use crate::regime;
use crate::scanner::BiographyFile;
use crate::surname::build_name_regex;
use crate::titles::build_title_regex;
use crate::types::{Book, Person, PersonKind};

// ── Byte span in a source file ───────────────────────────────────────

/// A byte range within a source file, for precise relocation.
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct TextSpan {
    pub file: String,
    pub byte_start: usize,
    pub byte_end: usize,
}

// ── Time reference ───────────────────────────────────────────────────

/// A time reference extracted from text, scoped to a regime.
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct TimeRef {
    pub era: String,
    pub regime: String,
    pub year: u8,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub month: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub day_ganzhi: Option<String>,
    /// Raw matched text
    pub raw: String,
    /// Byte offset where this time reference appears in the source file
    pub byte_offset: usize,
}

// ── Time scope ───────────────────────────────────────────────────────

/// The region of text governed by a single time reference.
/// Extends from the TimeRef's position to the next TimeRef (or EOF).
/// Querying a time period returns these scopes as file pointers.
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct TimeScope {
    pub time: TimeRef,
    pub span: TextSpan,
}

// ── Time index (queryable) ──────────────────────────────────────────

/// Corpus-wide index mapping time periods to file locations.
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct TimeIndex {
    pub scopes: Vec<TimeScope>,
}

impl TimeIndex {
    /// Query scopes matching a specific era name and optional year.
    pub fn query(&self, era: &str, year: Option<u8>) -> Vec<&TimeScope> {
        self.scopes
            .iter()
            .filter(|s| s.time.era == era && year.is_none_or(|y| s.time.year == y))
            .collect()
    }

    /// Query scopes matching a year range within one era.
    pub fn query_range(&self, era: &str, year_from: u8, year_to: u8) -> Vec<&TimeScope> {
        self.scopes
            .iter()
            .filter(|s| s.time.era == era && s.time.year >= year_from && s.time.year <= year_to)
            .collect()
    }

    /// Query scopes matching a regime name.
    pub fn query_regime(&self, regime: &str) -> Vec<&TimeScope> {
        self.scopes
            .iter()
            .filter(|s| s.time.regime == regime)
            .collect()
    }
}

// ── Timeline: full era-year inventory ───────────────────────────────

/// A single observed time point (era+year) in the corpus.
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct TimePoint {
    pub era: String,
    pub year: u8,
    pub occurrence_count: usize,
    /// Source files where this time point appears.
    pub files: Vec<String>,
}

/// All observed years for one era name under one regime.
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct EraTimeline {
    pub era: String,
    /// Years sorted ascending, each with occurrence counts.
    pub years: Vec<TimePoint>,
}

/// All eras observed for one regime.
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct RegimeTimeline {
    pub regime: String,
    pub eras: Vec<EraTimeline>,
}

/// Full corpus chronological inventory.
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct Timeline {
    pub regimes: Vec<RegimeTimeline>,
    /// Total distinct (regime, era, year) triples.
    pub total_time_points: usize,
}

impl Timeline {
    /// Build a timeline from all collected time scopes.
    pub fn from_scopes(scopes: &[TimeScope]) -> Self {
        // Collect: (regime, era, year) → set of files
        let mut map: HashMap<(String, String, u8), Vec<String>> = HashMap::new();
        for s in scopes {
            let key = (s.time.regime.clone(), s.time.era.clone(), s.time.year);
            let files = map.entry(key).or_default();
            let f = &s.span.file;
            if !files.contains(f) {
                files.push(f.clone());
            }
        }

        // Group by regime → era → years
        let mut regime_map: HashMap<String, HashMap<String, Vec<TimePoint>>> = HashMap::new();
        for ((regime, era, year), files) in &map {
            let era_map = regime_map.entry(regime.clone()).or_default();
            let years = era_map.entry(era.clone()).or_default();
            years.push(TimePoint {
                era: era.clone(),
                year: *year,
                occurrence_count: files.len(),
                files: files.clone(),
            });
        }

        let mut regimes: Vec<RegimeTimeline> = regime_map
            .into_iter()
            .map(|(regime, era_map)| {
                let mut eras: Vec<EraTimeline> = era_map
                    .into_iter()
                    .map(|(era, mut years)| {
                        years.sort_by_key(|tp| tp.year);
                        EraTimeline { era, years }
                    })
                    .collect();
                // Sort eras by position in ERA_NAMES (chronological within regime)
                eras.sort_by_key(|e| era_sort_key(&regime, &e.era));
                RegimeTimeline { regime, eras }
            })
            .collect();
        // Sort regimes by historical start year
        regimes.sort_by_key(|r| {
            regime::ERA_NAMES
                .iter()
                .find(|e| e.regime.as_chinese() == r.regime)
                .map(|e| e.regime.start_ad_year())
                .unwrap_or(9999)
        });

        let total = map.len();
        Timeline {
            regimes,
            total_time_points: total,
        }
    }
}

// ── Place reference ──────────────────────────────────────────────────

/// A place name extracted from appointment or military context.
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct PlaceRef {
    pub name: String,
    pub is_qiao: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role_suffix: Option<String>, // 刺史, 太守, etc.
}

// ── Event types ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum EventKind {
    /// 以X為Y — person appointed to a position (possibly at a place)
    Appointment {
        person: String,
        new_title: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        place: Option<PlaceRef>,
    },
    /// 拜/除/遷/轉/授/徵/封 X 為 Y — official transfer, promotion, or enfeoffment
    Promotion {
        person: String,
        /// The appointing/transferring verb (拜/除/遷/轉/授/徵/封)
        verb: String,
        new_title: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        place: Option<PlaceRef>,
    },
    /// X即位/踐祚/繼位 — throne accession
    Accession {
        person: String,
        /// The accession verb (即位/踐祚/繼位/即皇帝位)
        verb: String,
    },
    /// X攻/伐/克/陷Y — military action
    Battle {
        person: String,
        verb: String,
        target: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        target_place: Option<PlaceRef>,
    },
    /// X薨/卒/崩 — death
    Death { person: String, verb: String },
}

/// A single extracted event with optional time context.
#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct Event {
    pub kind: EventKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub time: Option<TimeRef>,
    pub source_file: String,
    /// Byte offset of the event match in the source file
    pub byte_offset: usize,
    pub context: String,
    /// All place references found in the event's context window.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub locations: Vec<PlaceRef>,
}

impl Event {
    /// Extract the person name from this event's kind.
    pub fn person_name(&self) -> &str {
        match &self.kind {
            EventKind::Appointment { person, .. }
            | EventKind::Promotion { person, .. }
            | EventKind::Accession { person, .. }
            | EventKind::Battle { person, .. }
            | EventKind::Death { person, .. } => person,
        }
    }

    /// Collect all location names from this event (structured + context).
    pub fn all_location_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.locations.iter().map(|l| l.name.as_str()).collect();
        match &self.kind {
            EventKind::Appointment { place: Some(p), .. }
            | EventKind::Promotion { place: Some(p), .. }
            | EventKind::Battle {
                target_place: Some(p),
                ..
            } => {
                names.push(p.name.as_str());
            }
            _ => {}
        }
        names
    }
}

// ── Aggregated output ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct EventStats {
    pub total_events: usize,
    pub appointments: usize,
    pub promotions: usize,
    pub accessions: usize,
    pub battles: usize,
    pub deaths: usize,
    pub unique_time_refs: usize,
    pub unique_places: usize,
    pub era_distribution: HashMap<String, usize>,
    pub top_places: Vec<(String, usize)>,
}

// ── Scanner ──────────────────────────────────────────────────────────

pub struct EventScanner {
    // Time extraction
    re_time: Regex,
    re_month_day: Regex,
    // Event extraction
    re_appointment: Regex,
    re_promotion: Regex,
    re_accession: Regex,
    re_battle: Regex,
    re_death: Regex,
    // Place extraction from titles
    re_place_title: Regex,
}

/// Single Chinese digit character → value 1–9.
fn cn_digit(c: char) -> Option<u8> {
    match c {
        '一' => Some(1),
        '二' => Some(2),
        '三' => Some(3),
        '四' => Some(4),
        '五' => Some(5),
        '六' => Some(6),
        '七' => Some(7),
        '八' => Some(8),
        '九' => Some(9),
        _ => None,
    }
}

/// Parse a Chinese cardinal number (元/一–九十九) → u8.
///
/// Handles: 元, 一–九, 十, 十一–十九, 二十–九十, 二十一–九十九.
/// Returns `None` for anything else, which causes the enclosing event to be
/// dropped rather than silently misdated.
pub(crate) fn parse_cn_number(s: &str) -> Option<u8> {
    if s == "元" {
        return Some(1);
    }
    let chars: Vec<char> = s.chars().collect();
    match chars.as_slice() {
        // 十 (= 10) — must come before the catch-all single-char arm
        ['十'] => Some(10),
        // 十D (11–19)
        ['十', d] => Some(10 + cn_digit(*d)?),
        // 一–九 (single non-十 digit)
        [c] => cn_digit(*c),
        // D十 (20, 30 … 90)
        [d, '十'] => Some(cn_digit(*d)? * 10),
        // D十D (21–99)
        [d1, '十', d2] => Some(cn_digit(*d1)? * 10 + cn_digit(*d2)?),
        _ => None,
    }
}

/// Parse a Chinese month name → month number (1–12).
///
/// Accepts plain months (正/一–十二/臘) and leap months with the 閏 prefix
/// (閏正 through 閏十二). Leap months are returned with their base month
/// number because the schema has no separate leap-month field yet.
pub(crate) fn parse_cn_month(s: &str) -> Option<u8> {
    match s {
        "正" | "一" => return Some(1),
        "臘" => return Some(12),
        _ => {}
    }
    // Strip optional 閏 prefix
    let base = s.strip_prefix('閏').unwrap_or(s);
    let base = if base == "正" { "一" } else { base };
    let m = parse_cn_number(base)?;
    if (1..=12).contains(&m) { Some(m) } else { None }
}

impl EventScanner {
    pub fn new(known_persons: &[Person]) -> Self {
        let era_re = regime::build_era_regex();
        let extra_surnames = collect_extra_surnames(known_persons);
        let name_re = build_name_regex(&extra_surnames);
        let title_re = build_title_regex();

        // Time: {era}{number}年
        // Captures: era name, year number (Chinese)
        let re_time = Regex::new(&format!("({era_re})(元|[一二三四五六七八九十]{{1,3}})年"))
            .expect("time regex");

        // Month + day: (正|二|...|十二)月(干支)
        let re_month_day = Regex::new(
            r"(正|閏?[一二三四五六七八九十]{1,2}|臘)月([甲乙丙丁戊己庚辛壬癸][子丑寅卯辰巳午未申酉戌亥])?"
        )
        .expect("month_day regex");

        // Appointment: 以{title?}{name}為{new_title}
        let re_appointment = Regex::new(&format!("以[^為]{{0,12}}({name_re})為([^，。]{{2,20}})"))
            .expect("appointment regex");

        // Battle: {name}{verb}{target}
        // Stop target at function words (於/于 = "at", 以 = "with") to avoid
        // capturing trailing place/person phrases as part of the target.
        let re_battle = Regex::new(&format!(
            "({name_re})(攻|伐|討|克|陷|寇|圍|襲)([^，。於于以]{{2,8}})"
        ))
        .expect("battle regex");

        // Death: {title?}{name}(薨|卒|崩|死)
        let re_death =
            Regex::new(&format!("(?:{title_re})?({name_re})(薨|卒|崩)")).expect("death regex");

        // Promotion / transfer / enfeoffment: 拜/除/遷/轉/授/徵/封 {name} 為 {title}
        // Anchored on 為 like the appointment regex so the name regex doesn't greedily
        // consume 為 as a given-name character.  Allows up to 10 chars of intervening
        // text (e.g. an honorary title before the person name: "拜太尉王進為…").
        let re_promotion = Regex::new(&format!(
            "(拜|除|遷|轉|授|徵|封)[^，。為]{{0,10}}({name_re})為([^，。]{{2,20}})"
        ))
        .expect("promotion regex");

        // Accession: {name} immediately followed by an accession verb
        let re_accession =
            Regex::new(&format!("({name_re})(即位|踐祚|繼位|即皇帝位)")).expect("accession regex");

        // Place in title: {place}(刺史|太守|...)
        // Exclude enumeration comma (、) and common punctuation to avoid matching
        // across title boundaries like "振威將軍、刺史"
        let re_place_title =
            Regex::new(r"(南?[^\s，。、以為]{2,4})(刺史|太守|內史)").expect("place_title regex");

        EventScanner {
            re_time,
            re_month_day,
            re_appointment,
            re_promotion,
            re_accession,
            re_battle,
            re_death,
            re_place_title,
        }
    }

    /// Extract all time references from a text.
    fn extract_times(&self, content: &str, book: Book) -> Vec<(usize, TimeRef)> {
        let mut times = Vec::new();

        for caps in self.re_time.captures_iter(content) {
            let full_match = caps.get(0).unwrap();
            let era = caps.get(1).unwrap().as_str();
            let year_str = caps.get(2).unwrap().as_str();

            let year = match parse_cn_number(year_str) {
                Some(y) => y,
                None => continue,
            };

            let regime =
                regime::resolve_era(era, book).unwrap_or_else(|| regime::default_regime(book));

            // Look for month/day after this time reference
            let after = &content[full_match.end()..];
            let (month, day_ganzhi) = if let Some(md) = self.re_month_day.captures(after) {
                // Only match if it's close (within ~10 chars)
                if md.get(0).unwrap().start() < 15 {
                    let m = md.get(1).map(|m| m.as_str()).and_then(parse_cn_month);
                    let d = md.get(2).map(|m| m.as_str().to_string());
                    (m, d)
                } else {
                    (None, None)
                }
            } else {
                (None, None)
            };

            times.push((
                full_match.start(),
                TimeRef {
                    era: era.to_string(),
                    regime: regime.as_chinese().to_string(),
                    year,
                    month,
                    day_ganzhi,
                    raw: full_match.as_str().to_string(),
                    byte_offset: full_match.start(),
                },
            ));
        }

        times
    }

    /// Extract all place references from a context string.
    /// Uses stricter validation than `extract_place_from_title` because context
    /// windows contain arbitrary prose that can produce false place matches.
    fn extract_places_from_context(&self, context: &str) -> Vec<PlaceRef> {
        let mut places = Vec::new();
        for caps in self.re_place_title.captures_iter(context) {
            if let Some(m) = caps.get(1) {
                let name = m.as_str().to_string();
                if !is_plausible_place(&name) {
                    continue;
                }
                let suffix = caps.get(2).map(|m| m.as_str().to_string());
                let is_qiao =
                    name.starts_with('南') && name.ends_with('州') && name.chars().count() >= 3;
                places.push(PlaceRef {
                    name,
                    is_qiao,
                    role_suffix: suffix,
                });
            }
        }
        places
    }

    /// Extract a place reference from a title string like "郢州刺史".
    /// Falls back to detecting bare administrative places like "梁州".
    fn extract_place_from_title(&self, title_str: &str) -> Option<PlaceRef> {
        // Primary: match "{place}{role_suffix}" pattern (e.g., "郢州刺史")
        if let Some(caps) = self.re_place_title.captures(title_str) {
            let place_name = caps.get(1)?.as_str().to_string();
            let suffix = caps.get(2).map(|m| m.as_str().to_string());

            let is_qiao = place_name.starts_with('南')
                && place_name.ends_with('州')
                && place_name.chars().count() >= 3;

            return Some(PlaceRef {
                name: place_name,
                is_qiao,
                role_suffix: suffix,
            });
        }

        // Fallback: bare administrative place (e.g., "梁州", "南兗州", "吳郡")
        let admin_suffixes: &[char] = &['州', '郡', '縣', '國'];
        let char_count = title_str.chars().count();
        if (2..=4).contains(&char_count)
            && let Some(last) = title_str.chars().last()
            && admin_suffixes.contains(&last)
            && is_plausible_place(title_str)
        {
            let is_qiao =
                title_str.starts_with('南') && title_str.ends_with('州') && char_count >= 3;
            return Some(PlaceRef {
                name: title_str.to_string(),
                is_qiao,
                role_suffix: None,
            });
        }

        None
    }

    /// Detect if a battle target string is a place name.
    fn detect_place_target(target: &str) -> Option<PlaceRef> {
        let geo_suffixes: &[char] = &[
            '州', '郡', '縣', '城', '關', '塞', '鎮', '壁', '山', '水', '河', '江', '池', '谷',
            '嶺', '津', '渡', '橋', '亭', '營', '壘',
        ];
        let last = target.chars().last()?;
        if geo_suffixes.contains(&last) {
            Some(PlaceRef {
                name: target.to_string(),
                is_qiao: target.starts_with('南')
                    && target.ends_with('州')
                    && target.chars().count() >= 3,
                role_suffix: None,
            })
        } else {
            None
        }
    }

    /// Find the closest preceding time reference for a given byte offset.
    fn find_time_context(times: &[(usize, TimeRef)], event_offset: usize) -> Option<TimeRef> {
        // Find the last time ref that appears BEFORE this event
        times
            .iter()
            .rev()
            .find(|(off, _)| *off < event_offset)
            .map(|(_, t)| t.clone())
    }

    /// Build time scopes from extracted time references.
    /// Each scope extends from one TimeRef to the next (or EOF).
    fn build_time_scopes(
        times: &[(usize, TimeRef)],
        content_len: usize,
        source_file: &str,
    ) -> Vec<TimeScope> {
        let mut scopes = Vec::new();
        for i in 0..times.len() {
            let (start, ref time) = times[i];
            let end = if i + 1 < times.len() {
                times[i + 1].0
            } else {
                content_len
            };
            scopes.push(TimeScope {
                time: time.clone(),
                span: TextSpan {
                    file: source_file.to_string(),
                    byte_start: start,
                    byte_end: end,
                },
            });
        }
        scopes
    }

    /// Scan a single file for events and time scopes.
    pub fn scan_file(
        &self,
        content: &str,
        book: Book,
        source_file: &str,
    ) -> (Vec<Event>, Vec<TimeScope>) {
        let mut events = Vec::new();
        let times = self.extract_times(content, book);
        let scopes = Self::build_time_scopes(&times, content.len(), source_file);

        // Appointments
        for caps in self.re_appointment.captures_iter(content) {
            let full = caps.get(0).unwrap();
            let person = caps.get(1).unwrap().as_str();
            let new_title = caps.get(2).unwrap().as_str();

            if crate::intext::is_false_positive_name(person) {
                continue;
            }

            let place = self.extract_place_from_title(new_title);
            let time = Self::find_time_context(&times, full.start());
            let context = extract_context(content, full.start(), 30);
            let locations = self.extract_places_from_context(&context);

            events.push(Event {
                kind: EventKind::Appointment {
                    person: person.to_string(),
                    new_title: new_title.trim().to_string(),
                    place,
                },
                time,
                source_file: source_file.to_string(),
                byte_offset: full.start(),
                context,
                locations,
            });
        }

        // Promotions / transfers / enfeoffments
        for caps in self.re_promotion.captures_iter(content) {
            let full = caps.get(0).unwrap();
            let verb = caps.get(1).unwrap().as_str();
            let person = caps.get(2).unwrap().as_str();
            let new_title = caps.get(3).unwrap().as_str();

            if crate::intext::is_false_positive_name(person) {
                continue;
            }

            let place = self.extract_place_from_title(new_title);
            let time = Self::find_time_context(&times, full.start());
            let context = extract_context(content, full.start(), 30);
            let locations = self.extract_places_from_context(&context);

            events.push(Event {
                kind: EventKind::Promotion {
                    person: person.to_string(),
                    verb: verb.to_string(),
                    new_title: new_title.trim().to_string(),
                    place,
                },
                time,
                source_file: source_file.to_string(),
                byte_offset: full.start(),
                context,
                locations,
            });
        }

        // Accessions
        for caps in self.re_accession.captures_iter(content) {
            let full = caps.get(0).unwrap();
            let person = caps.get(1).unwrap().as_str();
            let verb = caps.get(2).unwrap().as_str();

            if crate::intext::is_false_positive_name(person) {
                continue;
            }

            let time = Self::find_time_context(&times, full.start());
            let context = extract_context(content, full.start(), 30);
            let locations = self.extract_places_from_context(&context);

            events.push(Event {
                kind: EventKind::Accession {
                    person: person.to_string(),
                    verb: verb.to_string(),
                },
                time,
                source_file: source_file.to_string(),
                byte_offset: full.start(),
                context,
                locations,
            });
        }

        // Battles
        for caps in self.re_battle.captures_iter(content) {
            let full = caps.get(0).unwrap();
            let person = caps.get(1).unwrap().as_str();
            let verb = caps.get(2).unwrap().as_str();
            let target = caps.get(3).unwrap().as_str();

            if crate::intext::is_false_positive_name(person) {
                continue;
            }

            let target_place = Self::detect_place_target(target);
            let time = Self::find_time_context(&times, full.start());
            let context = extract_context(content, full.start(), 30);
            let locations = self.extract_places_from_context(&context);

            events.push(Event {
                kind: EventKind::Battle {
                    person: person.to_string(),
                    verb: verb.to_string(),
                    target: target.to_string(),
                    target_place,
                },
                time,
                source_file: source_file.to_string(),
                byte_offset: full.start(),
                context,
                locations,
            });
        }

        // Deaths
        for caps in self.re_death.captures_iter(content) {
            let full = caps.get(0).unwrap();
            let person = caps.get(1).unwrap().as_str();
            let verb = caps.get(2).unwrap().as_str();

            if crate::intext::is_false_positive_name(person) {
                continue;
            }

            let time = Self::find_time_context(&times, full.start());
            let context = extract_context(content, full.start(), 30);
            let locations = self.extract_places_from_context(&context);

            events.push(Event {
                kind: EventKind::Death {
                    person: person.to_string(),
                    verb: verb.to_string(),
                },
                time,
                source_file: source_file.to_string(),
                byte_offset: full.start(),
                context,
                locations,
            });
        }

        (events, scopes)
    }

    /// Scan the entire corpus.
    pub fn scan_corpus(&self, bio_files: &[BiographyFile]) -> (Vec<Event>, TimeIndex, EventStats) {
        let mut all_events = Vec::new();
        let mut all_scopes = Vec::new();
        let mut era_dist: HashMap<String, usize> = HashMap::new();
        let mut place_counts: HashMap<String, usize> = HashMap::new();
        let mut time_set = std::collections::HashSet::new();
        let mut appointments = 0usize;
        let mut promotions = 0usize;
        let mut accessions = 0usize;
        let mut battles = 0usize;
        let mut deaths = 0usize;

        for bio in bio_files {
            let content = match fs::read_to_string(&bio.path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            let (events, scopes) =
                self.scan_file(&content, bio.source.book, &bio.path.display().to_string());

            for e in &events {
                match &e.kind {
                    EventKind::Appointment { place, .. } => {
                        appointments += 1;
                        if let Some(p) = place {
                            *place_counts.entry(p.name.clone()).or_insert(0) += 1;
                        }
                    }
                    EventKind::Promotion { place, .. } => {
                        promotions += 1;
                        if let Some(p) = place {
                            *place_counts.entry(p.name.clone()).or_insert(0) += 1;
                        }
                    }
                    EventKind::Accession { .. } => {
                        accessions += 1;
                    }
                    EventKind::Battle { target_place, .. } => {
                        battles += 1;
                        if let Some(p) = target_place {
                            *place_counts.entry(p.name.clone()).or_insert(0) += 1;
                        }
                    }
                    EventKind::Death { .. } => {
                        deaths += 1;
                    }
                }
                if let Some(t) = &e.time {
                    *era_dist
                        .entry(format!("{}/{}", t.regime, t.era))
                        .or_insert(0) += 1;
                    time_set.insert(format!("{}{}", t.era, t.year));
                }
            }

            all_events.extend(events);
            all_scopes.extend(scopes);
        }

        let mut top_places: Vec<(String, usize)> = place_counts.into_iter().collect();
        top_places.sort_by(|a, b| b.1.cmp(&a.1));
        top_places.truncate(30);

        let stats = EventStats {
            total_events: all_events.len(),
            appointments,
            promotions,
            accessions,
            battles,
            deaths,
            unique_time_refs: time_set.len(),
            unique_places: top_places.len(),
            era_distribution: era_dist,
            top_places,
        };

        let time_index = TimeIndex { scopes: all_scopes };

        (all_events, time_index, stats)
    }
}

/// Return the index of an era name within ERA_NAMES for a given regime.
/// Used to sort eras chronologically within a regime.
pub fn era_sort_key(regime_chinese: &str, era_name: &str) -> usize {
    regime::ERA_NAMES
        .iter()
        .enumerate()
        .find(|(_, e)| e.name == era_name && e.regime.as_chinese() == regime_chinese)
        .map(|(i, _)| i)
        .unwrap_or(usize::MAX)
}

/// Compute the exact AD year for a time reference.
///
/// Uses the per-era `start_ad` from `ERA_NAMES` (scraped from Wikipedia)
/// to give a precise result: `start_ad + (year - 1)`.
pub fn exact_ad_year(regime_chinese: &str, era_name: &str, year: u8) -> Option<u16> {
    let entry = regime::ERA_NAMES
        .iter()
        .find(|e| e.regime.as_chinese() == regime_chinese && e.name == era_name)?;
    Some(entry.start_ad + (year as u16 - 1))
}

fn collect_extra_surnames(persons: &[Person]) -> Vec<String> {
    let mut surnames = std::collections::HashSet::new();
    for p in persons {
        match &p.kind {
            PersonKind::Official { surname, .. } | PersonKind::Ruler { surname, .. } => {
                surnames.insert(surname.clone());
            }
            PersonKind::Emperor {
                surname: Some(s), ..
            } => {
                surnames.insert(s.clone());
            }
            _ => {}
        }
    }
    surnames.into_iter().collect()
}

/// Check whether a string looks like a plausible administrative place name.
/// Used to filter context-extracted place names (before 刺史/太守/內史).
/// In Six Dynasties texts, administrative place names are 2-3 characters
/// (e.g., 荊州, 揚州, 南兗州). Longer matches from context windows are
/// almost always junk-prefixed (e.g., "攻暐洛州" where only "洛州" is real).
fn is_plausible_place(name: &str) -> bool {
    let char_count = name.chars().count();
    // Reject very short or very long names
    if !(2..=3).contains(&char_count) {
        return false;
    }
    // Reject bracket/annotation artifacts
    if name.contains('[') || name.contains(']') {
        return false;
    }
    // Reject names starting with characters that cannot begin a place name
    let first = name.chars().next().unwrap();
    let bad_starts: &[char] = &[
        '殺', '攻', '伐', '克', '陷', '討', '破', '逐', '執', // military verbs
        '使', '令', '遣', '命', '除', '拜', '遷', '轉', '授', // appointment verbs
        '乃', '又', '則', '其', '先', '亦', '再', '俄', '仍', // adverbs/connectives
        '兄', '弟', '父', '母', '叔', // kinship terms
        '偽', '僞', '故', '舊', '前', '後', '害', '盜', // modifiers/verbs
        '加', '領', '兼', '行', '代', '署', '出', '入', '功', // official action words
        '是', '走', '率', '擒', '獲', '斬', '在', '及', // misc verbs
        '與', '隨', '自', '累', '左', '右', '號', '詔', '贈', // misc
        '遙', '重', '衆', '勒', '從', '結', '更', '如', '乘', // misc
        '時', '方', '永', '爲', '歷', '曆', '瑗', '苗', '宋', // temporal/surnames/misc
    ];
    !bad_starts.contains(&first)
}

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
    window
        .lines()
        .find(|l| !l.is_empty())
        .unwrap_or(&window)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── parse_cn_number ──────────────────────────────────────────────

    #[test]
    fn test_parse_cn_number_basic_digits() {
        assert_eq!(parse_cn_number("元"), Some(1));
        assert_eq!(parse_cn_number("一"), Some(1));
        assert_eq!(parse_cn_number("二"), Some(2));
        assert_eq!(parse_cn_number("九"), Some(9));
    }

    #[test]
    fn test_parse_cn_number_tens() {
        assert_eq!(parse_cn_number("十"), Some(10));
        assert_eq!(parse_cn_number("十一"), Some(11));
        assert_eq!(parse_cn_number("十九"), Some(19));
        assert_eq!(parse_cn_number("二十"), Some(20));
        assert_eq!(parse_cn_number("三十"), Some(30));
    }

    #[test]
    fn test_parse_cn_number_beyond_thirty() {
        assert_eq!(parse_cn_number("三十一"), Some(31));
        assert_eq!(parse_cn_number("四十"), Some(40));
        assert_eq!(parse_cn_number("四十三"), Some(43));
        assert_eq!(parse_cn_number("五十"), Some(50));
        assert_eq!(parse_cn_number("六十"), Some(60));
        assert_eq!(parse_cn_number("九十九"), Some(99));
    }

    #[test]
    fn test_parse_cn_number_invalid() {
        assert_eq!(parse_cn_number(""), None);
        assert_eq!(parse_cn_number("百"), None);
        assert_eq!(parse_cn_number("太"), None);
    }

    // ── parse_cn_month ───────────────────────────────────────────────

    #[test]
    fn test_parse_cn_month_plain() {
        assert_eq!(parse_cn_month("正"), Some(1));
        assert_eq!(parse_cn_month("一"), Some(1));
        assert_eq!(parse_cn_month("六"), Some(6));
        assert_eq!(parse_cn_month("十"), Some(10));
        assert_eq!(parse_cn_month("十二"), Some(12));
        assert_eq!(parse_cn_month("臘"), Some(12));
    }

    #[test]
    fn test_parse_cn_month_leap_all() {
        // Leap months 閏正 through 閏十二 must all parse
        assert_eq!(parse_cn_month("閏正"), Some(1));
        assert_eq!(parse_cn_month("閏一"), Some(1));
        assert_eq!(parse_cn_month("閏二"), Some(2));
        assert_eq!(parse_cn_month("閏三"), Some(3));
        assert_eq!(parse_cn_month("閏四"), Some(4));
        assert_eq!(parse_cn_month("閏五"), Some(5));
        assert_eq!(parse_cn_month("閏六"), Some(6));
        assert_eq!(parse_cn_month("閏七"), Some(7));
        assert_eq!(parse_cn_month("閏八"), Some(8));
        assert_eq!(parse_cn_month("閏九"), Some(9));
        assert_eq!(parse_cn_month("閏十"), Some(10));
        assert_eq!(parse_cn_month("閏十一"), Some(11));
        assert_eq!(parse_cn_month("閏十二"), Some(12));
    }

    #[test]
    fn test_parse_cn_month_invalid() {
        assert_eq!(parse_cn_month("十三"), None); // month 13 is out of range
        assert_eq!(parse_cn_month(""), None);
    }

    // ── is_plausible_place ───────────────────────────────────────────

    #[test]
    fn test_is_plausible_place_valid() {
        assert!(is_plausible_place("郢州"));
        assert!(is_plausible_place("荊州"));
        assert!(is_plausible_place("建康"));
        assert!(is_plausible_place("南兗州")); // 3 chars, starts with 南
    }

    #[test]
    fn test_is_plausible_place_bad_start() {
        // Military / action verbs as first char → reject
        assert!(!is_plausible_place("殺人"));
        assert!(!is_plausible_place("攻城"));
        assert!(!is_plausible_place("克敵"));
    }

    #[test]
    fn test_is_plausible_place_length_bounds() {
        // Single char → reject
        assert!(!is_plausible_place("州"));
        // 4-char names → reject (context extraction uses a tighter 2–3 limit)
        assert!(!is_plausible_place("建康城外"));
    }

    // ── build_time_scopes ────────────────────────────────────────────

    fn dummy_time(era: &str, year: u8, offset: usize) -> (usize, TimeRef) {
        (
            offset,
            TimeRef {
                era: era.to_string(),
                regime: "劉宋".to_string(),
                year,
                month: None,
                day_ganzhi: None,
                raw: format!("{era}{year}年"),
                byte_offset: offset,
            },
        )
    }

    #[test]
    fn test_build_time_scopes_empty() {
        let scopes = EventScanner::build_time_scopes(&[], 100, "test.txt");
        assert!(scopes.is_empty());
    }

    #[test]
    fn test_build_time_scopes_single() {
        let times = vec![dummy_time("元嘉", 1, 5)];
        let scopes = EventScanner::build_time_scopes(&times, 100, "test.txt");
        assert_eq!(scopes.len(), 1);
        assert_eq!(scopes[0].span.byte_start, 5);
        assert_eq!(scopes[0].span.byte_end, 100); // extends to EOF
        assert_eq!(scopes[0].time.era, "元嘉");
    }

    #[test]
    fn test_build_time_scopes_multiple() {
        let times = vec![
            dummy_time("元嘉", 1, 10),
            dummy_time("元嘉", 5, 50),
            dummy_time("元嘉", 10, 90),
        ];
        let scopes = EventScanner::build_time_scopes(&times, 200, "f.txt");
        assert_eq!(scopes.len(), 3);
        assert_eq!(scopes[0].span.byte_start, 10);
        assert_eq!(scopes[0].span.byte_end, 50); // ends where next starts
        assert_eq!(scopes[1].span.byte_start, 50);
        assert_eq!(scopes[1].span.byte_end, 90);
        assert_eq!(scopes[2].span.byte_start, 90);
        assert_eq!(scopes[2].span.byte_end, 200); // last extends to EOF
    }

    // ── find_time_context ────────────────────────────────────────────

    #[test]
    fn test_find_time_context_none_when_no_preceding() {
        let times = vec![dummy_time("元嘉", 1, 50)];
        // Event is before the first time ref
        assert!(EventScanner::find_time_context(&times, 10).is_none());
    }

    #[test]
    fn test_find_time_context_returns_last_preceding() {
        let times = vec![
            dummy_time("元嘉", 1, 10),
            dummy_time("元嘉", 5, 30),
            dummy_time("元嘉", 10, 80),
        ];
        // Event at offset 60 — last preceding time is at 30
        let t = EventScanner::find_time_context(&times, 60).unwrap();
        assert_eq!(t.year, 5);
    }

    #[test]
    fn test_find_time_context_after_all() {
        let times = vec![dummy_time("元嘉", 1, 10), dummy_time("元嘉", 5, 30)];
        // Event after all times → picks the last one (year 5)
        let t = EventScanner::find_time_context(&times, 999).unwrap();
        assert_eq!(t.year, 5);
    }

    // ── Regex-level event extraction ─────────────────────────────────

    /// Build an EventScanner with no known persons (uses default surname list).
    fn scanner() -> EventScanner {
        EventScanner::new(&[])
    }

    #[test]
    fn test_scan_file_appointment_detected() {
        let s = scanner();
        // 以X為Y pattern — 王 is a common surname in the default list
        let text = "以王進為冠軍將軍。";
        let (events, _) = s.scan_file(text, crate::types::Book::SongShu, "test.txt");
        let appts: Vec<_> = events
            .iter()
            .filter(|e| matches!(&e.kind, EventKind::Appointment { .. }))
            .collect();
        assert!(!appts.is_empty(), "should detect at least one appointment");
        if let EventKind::Appointment { new_title, .. } = &appts[0].kind {
            assert!(new_title.contains("冠軍"), "title should contain 冠軍");
        }
    }

    #[test]
    fn test_scan_file_promotion_detected() {
        let s = scanner();
        // 拜X為Y pattern
        let text = "拜王進為益州刺史，入朝。";
        let (events, _) = s.scan_file(text, crate::types::Book::SongShu, "test.txt");
        let proms: Vec<_> = events
            .iter()
            .filter(|e| matches!(&e.kind, EventKind::Promotion { .. }))
            .collect();
        assert!(!proms.is_empty(), "should detect at least one promotion");
        if let EventKind::Promotion { verb, .. } = &proms[0].kind {
            assert_eq!(verb, "拜");
        }
    }

    #[test]
    fn test_scan_file_promotion_verbs() {
        let s = scanner();
        for verb in &["拜", "除", "遷", "授", "徵", "封"] {
            let text = format!("{verb}王進為太守，出鎮。");
            let (events, _) = s.scan_file(&text, crate::types::Book::SongShu, "test.txt");
            let proms: Vec<_> = events
                .iter()
                .filter(|e| matches!(&e.kind, EventKind::Promotion { .. }))
                .collect();
            assert!(
                !proms.is_empty(),
                "verb '{verb}' should produce at least one promotion event"
            );
        }
    }

    #[test]
    fn test_scan_file_accession_detected() {
        let s = scanner();
        let text = "王進即位，改元建平。";
        let (events, _) = s.scan_file(text, crate::types::Book::SongShu, "test.txt");
        let acc: Vec<_> = events
            .iter()
            .filter(|e| matches!(&e.kind, EventKind::Accession { .. }))
            .collect();
        assert!(!acc.is_empty(), "should detect accession event");
        if let EventKind::Accession { verb, .. } = &acc[0].kind {
            assert_eq!(verb, "即位");
        }
    }

    #[test]
    fn test_scan_file_battle_detected() {
        let s = scanner();
        let text = "王進攻建康城，克之。";
        let (events, _) = s.scan_file(text, crate::types::Book::SongShu, "test.txt");
        let battles: Vec<_> = events
            .iter()
            .filter(|e| matches!(&e.kind, EventKind::Battle { .. }))
            .collect();
        assert!(!battles.is_empty(), "should detect battle event");
        if let EventKind::Battle { verb, target, .. } = &battles[0].kind {
            assert_eq!(verb, "攻");
            assert!(target.contains("建康"), "target should contain 建康");
        }
    }

    #[test]
    fn test_scan_file_death_detected() {
        let s = scanner();
        let text = "王進卒，時年五十。";
        let (events, _) = s.scan_file(text, crate::types::Book::SongShu, "test.txt");
        let deaths: Vec<_> = events
            .iter()
            .filter(|e| matches!(&e.kind, EventKind::Death { .. }))
            .collect();
        assert!(!deaths.is_empty(), "should detect death event");
        if let EventKind::Death { verb, .. } = &deaths[0].kind {
            assert_eq!(verb, "卒");
        }
    }

    #[test]
    fn test_scan_file_time_scope_attached() {
        let s = scanner();
        // Time reference before the event should be attached
        let text = "元嘉三年，以王進為冠軍將軍。";
        let (events, scopes) = s.scan_file(text, crate::types::Book::SongShu, "test.txt");
        assert!(!scopes.is_empty(), "should have at least one time scope");
        let appts: Vec<_> = events
            .iter()
            .filter(|e| matches!(&e.kind, EventKind::Appointment { .. }))
            .collect();
        assert!(!appts.is_empty());
        let t = appts[0].time.as_ref().expect("event should have time ref");
        assert_eq!(t.era, "元嘉");
        assert_eq!(t.year, 3);
    }

    #[test]
    fn test_parse_cn_number_high_year_in_scan() {
        let s = scanner();
        // Year 四十三 (43) should be parsed correctly, not dropped
        let text = "元嘉四十三年，王進卒。";
        let (_, scopes) = s.scan_file(text, crate::types::Book::SongShu, "test.txt");
        // If parse_cn_number handles >30, we should get a scope
        assert!(
            !scopes.is_empty(),
            "year 四十三 should produce a time scope"
        );
        assert_eq!(scopes[0].time.year, 43);
    }

    // ── exact_ad_year ────────────────────────────────────────────────

    #[test]
    fn test_exact_ad_year_liu_song() {
        // 元嘉 started in AD 424
        assert_eq!(exact_ad_year("劉宋", "元嘉", 1), Some(424));
        assert_eq!(exact_ad_year("劉宋", "元嘉", 30), Some(453));
    }

    #[test]
    fn test_exact_ad_year_northern_wei() {
        // 太和 started in AD 477
        assert_eq!(exact_ad_year("北魏", "太和", 1), Some(477));
        assert_eq!(exact_ad_year("北魏", "太和", 23), Some(499));
    }

    #[test]
    fn test_exact_ad_year_cross_regime_ordering() {
        // 劉宋/元嘉 (AD 424) should be BEFORE 北魏/太和 (AD 477)
        let song = exact_ad_year("劉宋", "元嘉", 1).unwrap();
        let wei = exact_ad_year("北魏", "太和", 1).unwrap();
        assert!(
            song < wei,
            "劉宋/元嘉({song}) should be before 北魏/太和({wei})"
        );
    }

    #[test]
    fn test_exact_ad_year_dynasty_succession() {
        // 義熙14年 = AD 418, 元嘉1年 = AD 424 — gap of 6 years (劉宋永初 in between)
        let yixi = exact_ad_year("東晉", "義熙", 14).unwrap();
        let yuanjia = exact_ad_year("劉宋", "元嘉", 1).unwrap();
        assert_eq!(yixi, 418);
        assert_eq!(yuanjia, 424);
        let gap = yuanjia.abs_diff(yixi);
        assert!(gap <= 10, "義熙14→元嘉1 gap should be small, got {gap}");
    }

    #[test]
    fn test_exact_ad_year_unknown_era() {
        assert!(exact_ad_year("劉宋", "不存在", 1).is_none());
    }

    #[test]
    fn test_exact_ad_year_unknown_regime() {
        assert!(exact_ad_year("不存在", "元嘉", 1).is_none());
    }
}
