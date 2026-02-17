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

// ── Aggregated output ────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, serde::Deserialize)]
pub struct EventStats {
    pub total_events: usize,
    pub appointments: usize,
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
    re_battle: Regex,
    re_death: Regex,
    // Place extraction from titles
    re_place_title: Regex,
}

/// Chinese number word → digit
fn parse_cn_number(s: &str) -> Option<u8> {
    match s {
        "元" => Some(1),
        "一" => Some(1),
        "二" => Some(2),
        "三" => Some(3),
        "四" => Some(4),
        "五" => Some(5),
        "六" => Some(6),
        "七" => Some(7),
        "八" => Some(8),
        "九" => Some(9),
        "十" => Some(10),
        "十一" => Some(11),
        "十二" => Some(12),
        "十三" => Some(13),
        "十四" => Some(14),
        "十五" => Some(15),
        "十六" => Some(16),
        "十七" => Some(17),
        "十八" => Some(18),
        "十九" => Some(19),
        "二十" => Some(20),
        "二十一" => Some(21),
        "二十二" => Some(22),
        "二十三" => Some(23),
        "二十四" => Some(24),
        "二十五" => Some(25),
        "二十六" => Some(26),
        "二十七" => Some(27),
        "二十八" => Some(28),
        "二十九" => Some(29),
        "三十" => Some(30),
        _ => None,
    }
}

fn parse_cn_month(s: &str) -> Option<u8> {
    match s {
        "正" => Some(1),
        "一" => Some(1),
        "二" => Some(2),
        "三" => Some(3),
        "四" => Some(4),
        "五" => Some(5),
        "六" => Some(6),
        "七" => Some(7),
        "八" => Some(8),
        "九" => Some(9),
        "十" => Some(10),
        "十一" => Some(11),
        "十二" | "臘" => Some(12),
        "閏正" | "閏一" => Some(1),
        "閏二" => Some(2),
        "閏三" => Some(3),
        _ => None,
    }
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
        let re_battle = Regex::new(&format!(
            "({name_re})(攻|伐|討|克|陷|寇|圍|襲)([^，。]{{2,8}})"
        ))
        .expect("battle regex");

        // Death: {title?}{name}(薨|卒|崩|死)
        let re_death =
            Regex::new(&format!("(?:{title_re})?({name_re})(薨|卒|崩)")).expect("death regex");

        // Place in title: {place}(刺史|太守|...)
        let re_place_title =
            Regex::new(r"(南?[^\s，。以為]{1,4})(刺史|太守|內史)").expect("place_title regex");

        EventScanner {
            re_time,
            re_month_day,
            re_appointment,
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
    fn extract_places_from_context(&self, context: &str) -> Vec<PlaceRef> {
        let mut places = Vec::new();
        for caps in self.re_place_title.captures_iter(context) {
            if let Some(m) = caps.get(1) {
                let name = m.as_str().to_string();
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
    fn extract_place_from_title(&self, title_str: &str) -> Option<PlaceRef> {
        if let Some(caps) = self.re_place_title.captures(title_str) {
            let place_name = caps.get(1)?.as_str().to_string();
            let suffix = caps.get(2).map(|m| m.as_str().to_string());

            // Detect 僑制: "南X州" pattern
            let is_qiao = place_name.starts_with('南')
                && place_name.ends_with('州')
                && place_name.chars().count() >= 3;

            Some(PlaceRef {
                name: place_name,
                is_qiao,
                role_suffix: suffix,
            })
        } else {
            None
        }
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
fn era_sort_key(regime_chinese: &str, era_name: &str) -> usize {
    regime::ERA_NAMES
        .iter()
        .enumerate()
        .find(|(_, e)| e.name == era_name && e.regime.as_chinese() == regime_chinese)
        .map(|(i, _)| i)
        .unwrap_or(usize::MAX)
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
