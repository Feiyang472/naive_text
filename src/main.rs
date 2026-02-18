mod event;
mod extract;
mod intext;
mod parser;
mod regime;
mod scanner;
mod surname;
mod titles;
mod types;

use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand};
use extract::PersonSummary;
use types::Section;

const OUTPUT_DIR: &str = "output";

#[derive(Parser)]
#[command(
    name = "person_extract",
    about = "Six Dynasties historical text analyzer"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Run full corpus extraction → output/*.json
    Extract {
        /// Path to corpus root directory
        #[arg(default_value = ".")]
        corpus: PathBuf,
    },
    /// Query time periods from cached output
    Query {
        /// Time query, e.g. "太和三年", "太和元年-太和六年", "@東晉"
        query: Vec<String>,
    },
    /// Print the full era-year timeline inventory
    Timeline,
    /// Extract source text for a time period
    Text {
        /// Time query, e.g. "太和三年", "太和元年-太和六年", "@東晉"
        query: Vec<String>,
    },
    /// Map persons to locations for a time period
    Locate {
        /// Time query, e.g. "太和三年", "元嘉", "@東晉"
        query: Vec<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Command::Extract { corpus }) => run_extract(&corpus),
        Some(Command::Query { query }) => run_query(&query),
        Some(Command::Timeline) => run_timeline(),
        Some(Command::Text { query }) => run_text(&query),
        Some(Command::Locate { query }) => run_locate(&query),
        // Default: extract from current directory
        None => run_extract(Path::new(".")),
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  OUTPUT FILE HELPERS
// ═══════════════════════════════════════════════════════════════════════

fn output_path(name: &str) -> PathBuf {
    Path::new(OUTPUT_DIR).join(name)
}

fn write_json<T: serde::Serialize>(name: &str, data: &T) {
    let path = output_path(name);
    let json = serde_json::to_string_pretty(data).expect("JSON serialization failed");
    std::fs::write(&path, &json).unwrap_or_else(|e| panic!("cannot write {}: {e}", path.display()));
    eprintln!("  {} ({} bytes)", path.display(), json.len());
}

fn read_json<T: serde::de::DeserializeOwned>(name: &str) -> T {
    let path = output_path(name);
    let json = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        eprintln!("Cannot read {}: {e}", path.display());
        eprintln!("Run extraction first (without --query) to generate the index.");
        std::process::exit(1);
    });
    serde_json::from_str(&json).unwrap_or_else(|e| {
        eprintln!("Cannot parse {}: {e}", path.display());
        eprintln!("The JSON may be from an older format. Re-run extraction.");
        std::process::exit(1);
    })
}

// ═══════════════════════════════════════════════════════════════════════
//  TIMELINE MODE: print the era-year inventory to stdout
// ═══════════════════════════════════════════════════════════════════════

#[derive(serde::Deserialize)]
struct TimelineFile {
    timeline: event::Timeline,
    #[allow(dead_code)]
    time_index: event::TimeIndex,
    #[allow(dead_code)]
    stats: event::EventStats,
}

fn run_timeline() {
    let data: TimelineFile = read_json("timeline.json");

    // Build lookup: regime Chinese name → RegimeTimeline
    let regime_map: std::collections::HashMap<&str, &event::RegimeTimeline> = data
        .timeline
        .regimes
        .iter()
        .map(|r| (r.regime.as_str(), r))
        .collect();

    let tree = regime::regime_display_tree();
    render_tree(&tree, &regime_map, "");

    eprintln!(
        "\nTotal: {} distinct (regime, era, year) triples",
        data.timeline.total_time_points
    );
}

/// Format a regime line: "name (start_year): era1, era2, ..."
/// Returns None if the regime has no observed data in the corpus.
fn format_regime_line(
    r: regime::Regime,
    regime_map: &std::collections::HashMap<&str, &event::RegimeTimeline>,
) -> Option<String> {
    let name = r.as_chinese();
    let rt = regime_map.get(name)?;
    let eras: Vec<&str> = rt.eras.iter().map(|e| e.era.as_str()).collect();
    Some(format!(
        "{} ({}): {}",
        name,
        r.start_ad_year(),
        eras.join(", ")
    ))
}

/// Recursively render the display tree with box-drawing characters.
///
/// `prefix` is the accumulated line-drawing prefix for the current depth.
fn render_tree(
    node: &regime::DisplayTree,
    regime_map: &std::collections::HashMap<&str, &event::RegimeTimeline>,
    prefix: &str,
) {
    let (regime, concurrent, sequence) = match node {
        regime::DisplayTree::Leaf(r) => (*r, &[][..], &[][..]),
        regime::DisplayTree::Branch {
            regime,
            concurrent,
            sequence,
        } => (*regime, concurrent.as_slice(), sequence.as_slice()),
    };

    // Print this node's regime
    if format_regime_line(regime, regime_map).is_none() {
        return;
    }
    println!(
        "{}{}",
        prefix,
        format_regime_line(regime, regime_map).unwrap()
    );

    // Collect children: concurrent first (with [並立] markers), then sequential
    let has_seq = !sequence.is_empty();
    let total_conc = concurrent.len();

    for (i, child) in concurrent.iter().enumerate() {
        let is_last = i == total_conc - 1 && !has_seq;
        let connector = if is_last { "└─ " } else { "├─ " };
        let continuation = if is_last { "   " } else { "│  " };

        let child_regime = match child {
            regime::DisplayTree::Leaf(r) | regime::DisplayTree::Branch { regime: r, .. } => *r,
        };
        if let Some(line) = format_regime_line(child_regime, regime_map) {
            println!("{}{}[並立] {}", prefix, connector, line);
        } else {
            continue;
        }

        // Recurse into sub-children if this child is a Branch
        if let regime::DisplayTree::Branch {
            concurrent: sub_conc,
            sequence: sub_seq,
            ..
        } = child
        {
            let sub_prefix = format!("{}{}", prefix, continuation);
            render_children(sub_conc, sub_seq, &sub_prefix, regime_map);
        }
    }

    // Sequential successors continue on the same vertical line
    if has_seq {
        let seq_prefix = if prefix.is_empty() {
            String::new()
        } else {
            format!("{}│  ", prefix)
        };
        render_children(&[], sequence, &seq_prefix, regime_map);
    }
}

/// Render a list of concurrent branches + sequential children at a given prefix.
fn render_children(
    concurrent: &[regime::DisplayTree],
    sequence: &[regime::DisplayTree],
    prefix: &str,
    regime_map: &std::collections::HashMap<&str, &event::RegimeTimeline>,
) {
    let has_seq = !sequence.is_empty();
    let total_conc = concurrent.len();

    for (i, child) in concurrent.iter().enumerate() {
        let is_last = i == total_conc - 1 && !has_seq;
        let connector = if is_last { "└─ " } else { "├─ " };
        let continuation = if is_last { "   " } else { "│  " };

        let child_regime = match child {
            regime::DisplayTree::Leaf(r) | regime::DisplayTree::Branch { regime: r, .. } => *r,
        };
        if let Some(line) = format_regime_line(child_regime, regime_map) {
            println!("{}{}[並立] {}", prefix, connector, line);
        } else {
            continue;
        }

        if let regime::DisplayTree::Branch {
            concurrent: sub_conc,
            sequence: sub_seq,
            ..
        } = child
        {
            let sub_prefix = format!("{}{}", prefix, continuation);
            render_children(sub_conc, sub_seq, &sub_prefix, regime_map);
        }
    }

    for child in sequence {
        render_tree(child, regime_map, prefix);
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  QUERY MODE: read cached JSONs, return matching scopes + events
// ═══════════════════════════════════════════════════════════════════════

/// Deserialization wrapper for the new events.json format.
#[derive(serde::Deserialize)]
struct EventsFile {
    events: Vec<event::Event>,
    #[allow(dead_code)]
    unstructured_events: Vec<event::Event>,
}

fn run_query(query_args: &[String]) {
    let raw = query_args.join(" ");

    let timeline_data: TimelineFile = read_json("timeline.json");
    let events_file: EventsFile = read_json("events.json");
    let events = events_file.events;

    // Parse query: "太和", "太和三年", "太和元年-太和六年", "太和1-5"
    let parsed = parse_time_query(&raw);

    let matching_scopes = match &parsed {
        TimeQuery::Single { era, year } => timeline_data.time_index.query(era, *year),
        TimeQuery::Range {
            era,
            year_from,
            year_to,
        } => timeline_data
            .time_index
            .query_range(era, *year_from, *year_to),
        TimeQuery::Regime { regime } => timeline_data.time_index.query_regime(regime),
    };

    if matching_scopes.is_empty() {
        eprintln!("No time scopes found for: {raw}");
        eprintln!("  parsed as: {parsed:?}");
        // Show available eras
        let mut eras: Vec<&str> = timeline_data
            .time_index
            .scopes
            .iter()
            .map(|s| s.time.era.as_str())
            .collect();
        eras.sort();
        eras.dedup();
        eprintln!("  available eras: {}", eras.join(", "));
        return;
    }

    // Filter events: find events whose time matches the query
    let matching_events: Vec<&event::Event> = events
        .iter()
        .filter(|e| {
            if let Some(t) = &e.time {
                match &parsed {
                    TimeQuery::Single { era, year } => {
                        t.era == *era && year.is_none_or(|y| t.year == y)
                    }
                    TimeQuery::Range {
                        era,
                        year_from,
                        year_to,
                    } => t.era == *era && t.year >= *year_from && t.year <= *year_to,
                    TimeQuery::Regime { regime } => t.regime == *regime,
                }
            } else {
                false
            }
        })
        .collect();

    eprintln!(
        "Found {} time scope(s), {} event(s) for: {}",
        matching_scopes.len(),
        matching_events.len(),
        raw
    );

    // Output to stdout
    #[derive(serde::Serialize)]
    struct QueryResult<'a> {
        query: String,
        scope_count: usize,
        event_count: usize,
        scopes: Vec<&'a event::TimeScope>,
        events: Vec<&'a event::Event>,
    }

    let result = QueryResult {
        query: raw,
        scope_count: matching_scopes.len(),
        event_count: matching_events.len(),
        scopes: matching_scopes,
        events: matching_events,
    };

    let json = serde_json::to_string_pretty(&result).expect("JSON serialization");
    println!("{json}");
}

// ── Query parsing ───────────────────────────────────────────────────

#[derive(Debug)]
enum TimeQuery {
    /// Single era + optional year: "太和", "太和三年"
    Single { era: String, year: Option<u8> },
    /// Year range within one era: "太和元年-太和六年", "太和1-5"
    Range {
        era: String,
        year_from: u8,
        year_to: u8,
    },
    /// All scopes for a regime: "@東晉", "@北魏"
    Regime { regime: String },
}

fn parse_time_query(raw: &str) -> TimeQuery {
    let raw = raw.trim();

    // Regime query: "@東晉"
    if let Some(r) = raw.strip_prefix('@') {
        return TimeQuery::Regime {
            regime: r.to_string(),
        };
    }

    // Range with dash: "太和元年-太和六年" or "太和1-5" or "太和元-六"
    if raw.contains('-') || raw.contains('—') || raw.contains('~') {
        let sep = if raw.contains('-') {
            '-'
        } else if raw.contains('—') {
            '—'
        } else {
            '~'
        };
        let parts: Vec<&str> = raw.splitn(2, sep).collect();
        if parts.len() == 2 {
            let (era_from, year_from) = parse_era_year(parts[0]);
            let (era_to, year_to) = parse_era_year(parts[1]);

            if let (Some(yf), Some(yt)) = (year_from, year_to) {
                // Use the era from the first part (or second if first is just a number)
                let era = if era_from.is_empty() {
                    era_to
                } else {
                    era_from
                };
                return TimeQuery::Range {
                    era,
                    year_from: yf,
                    year_to: yt,
                };
            }
        }
    }

    // Single: "太和三年", "太和3", "太和"
    let (era, year) = parse_era_year(raw);
    TimeQuery::Single { era, year }
}

/// Parse "太和三年" → ("太和", Some(3)), "太和" → ("太和", None), "5" → ("", Some(5))
fn parse_era_year(raw: &str) -> (String, Option<u8>) {
    let raw = raw.trim().trim_end_matches('年');

    // Pure Arabic number: "5"
    if let Ok(y) = raw.parse::<u8>() {
        return (String::new(), Some(y));
    }

    // Trailing Arabic number: "太和3"
    if let Some(idx) = raw.rfind(|c: char| !c.is_ascii_digit()) {
        let char_end = idx + raw[idx..].chars().next().unwrap().len_utf8();
        let after = &raw[char_end..];
        if !after.is_empty()
            && let Ok(y) = after.parse::<u8>()
        {
            return (raw[..char_end].to_string(), Some(y));
        }
    }

    // Chinese number suffix: "太和三", "太和十二", "太和元"
    let chars: Vec<char> = raw.chars().collect();
    for suffix_len in (1..=3).rev() {
        if chars.len() <= suffix_len {
            continue;
        }
        let suffix: String = chars[chars.len() - suffix_len..].iter().collect();
        if let Some(y) = parse_cn_year(&suffix) {
            let era: String = chars[..chars.len() - suffix_len].iter().collect();
            if !era.is_empty() {
                return (era, Some(y));
            }
        }
    }

    (raw.to_string(), None)
}

fn parse_cn_year(s: &str) -> Option<u8> {
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
        _ => None,
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  TEXT MODE: extract source text for a time period
// ═══════════════════════════════════════════════════════════════════════

fn run_text(query_args: &[String]) {
    let raw = query_args.join(" ");
    let timeline_data: TimelineFile = read_json("timeline.json");
    let parsed = parse_time_query(&raw);

    let matching_scopes = match &parsed {
        TimeQuery::Single { era, year } => timeline_data.time_index.query(era, *year),
        TimeQuery::Range {
            era,
            year_from,
            year_to,
        } => timeline_data
            .time_index
            .query_range(era, *year_from, *year_to),
        TimeQuery::Regime { regime } => timeline_data.time_index.query_regime(regime),
    };

    if matching_scopes.is_empty() {
        eprintln!("No time scopes found for: {raw}");
        return;
    }

    eprintln!("Found {} text scope(s) for: {}", matching_scopes.len(), raw);

    // Group scopes by file to avoid re-reading
    let mut by_file: std::collections::HashMap<&str, Vec<&event::TimeScope>> =
        std::collections::HashMap::new();
    for scope in &matching_scopes {
        by_file
            .entry(scope.span.file.as_str())
            .or_default()
            .push(scope);
    }

    for (file, scopes) in &by_file {
        let content = match std::fs::read_to_string(file) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Cannot read {}: {}", file, e);
                continue;
            }
        };

        for scope in scopes {
            let start = scope.span.byte_start.min(content.len());
            let end = scope.span.byte_end.min(content.len());
            let text = &content[start..end];
            if text.trim().is_empty() {
                continue;
            }

            println!(
                "── [{}/{}{}年] {} ──",
                scope.time.regime, scope.time.era, scope.time.year, file
            );
            println!("{}", text.trim());
            println!();
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  LOCATE MODE: map persons to locations for a time period
// ═══════════════════════════════════════════════════════════════════════

/// Chronological sort key using exact AD year.
/// Globally comparable across regimes.
fn time_sort_key(t: &event::TimeRef) -> u16 {
    event::exact_ad_year(&t.regime, &t.era, t.year).unwrap_or(0)
}

/// Check whether a time point matches the parsed query.
fn time_matches_query(t: &event::TimeRef, parsed: &TimeQuery) -> bool {
    match parsed {
        TimeQuery::Single { era, year } => t.era == *era && year.is_none_or(|y| t.year == y),
        TimeQuery::Range {
            era,
            year_from,
            year_to,
        } => t.era == *era && t.year >= *year_from && t.year <= *year_to,
        TimeQuery::Regime { regime } => t.regime == *regime,
    }
}

/// Check if a person is "stale" — last seen more than ~30 AD years ago.
const STALENESS_THRESHOLD_YEARS: u16 = 30;

fn is_stale(last_seen_ad: u16, query_ad: u16) -> bool {
    query_ad.saturating_sub(last_seen_ad) > STALENESS_THRESHOLD_YEARS
}

fn run_locate(query_args: &[String]) {
    let raw = query_args.join(" ");
    let events_file: EventsFile = read_json("events.json");
    let events = events_file.events;
    let parsed = parse_time_query(&raw);

    // Only process events that have time references
    let mut timed_events: Vec<&event::Event> = events.iter().filter(|e| e.time.is_some()).collect();

    // Sort all events chronologically by approximate AD year
    timed_events.sort_by_key(|e| time_sort_key(e.time.as_ref().unwrap()));

    // Determine the query time range for filtering output
    let query_max_key: Option<u16> = timed_events
        .iter()
        .filter(|e| time_matches_query(e.time.as_ref().unwrap(), &parsed))
        .map(|e| time_sort_key(e.time.as_ref().unwrap()))
        .max();

    let query_max_key = match query_max_key {
        Some(k) => k,
        None => {
            eprintln!("No events found for: {raw}");
            return;
        }
    };

    // Walk events in chronological order, building per-person state
    struct PersonState {
        location: Option<LocRecord>,
        last_seen: u16,
        dead_at: Option<u16>,
    }

    struct LocRecord {
        place: String,
        role: Option<String>,
        as_of: String,
        ad_year: u16,
    }

    let mut state: std::collections::HashMap<String, PersonState> =
        std::collections::HashMap::new();

    for e in &timed_events {
        let t = e.time.as_ref().unwrap();
        let key = time_sort_key(t);

        // Stop processing events beyond the query cutoff
        if key > query_max_key {
            break;
        }

        let time_label = format!("{}/{}{}年", t.regime, t.era, t.year);

        // Extract person name from event
        let person = match &e.kind {
            event::EventKind::Appointment { person, .. }
            | event::EventKind::Battle { person, .. }
            | event::EventKind::Death { person, .. } => person.clone(),
        };

        let ps = state.entry(person).or_insert(PersonState {
            location: None,
            last_seen: key,
            dead_at: None,
        });

        // Update last seen
        ps.last_seen = key;

        // Update location from structured place fields
        let mut has_structured_place = false;
        match &e.kind {
            event::EventKind::Appointment {
                place: Some(place),
                new_title,
                ..
            } => {
                has_structured_place = true;
                ps.location = Some(LocRecord {
                    place: place.name.clone(),
                    role: Some(new_title.clone()),
                    as_of: time_label.clone(),
                    ad_year: key,
                });
            }
            event::EventKind::Battle {
                target_place: Some(place),
                ..
            } => {
                has_structured_place = true;
                ps.location = Some(LocRecord {
                    place: place.name.clone(),
                    role: None,
                    as_of: time_label.clone(),
                    ad_year: key,
                });
            }
            event::EventKind::Death { .. } => {
                ps.dead_at = Some(key);
            }
            _ => {}
        }

        // Fall back to context locations when no structured place was set
        if !has_structured_place && let Some(loc) = e.locations.first() {
            ps.location = Some(LocRecord {
                place: loc.name.clone(),
                role: loc.role_suffix.clone(),
                as_of: time_label.clone(),
                ad_year: key,
            });
        }
    }

    // Build output: filter to persons with known location, not dead, not stale
    let mut result: std::collections::HashMap<String, PersonLocation> =
        std::collections::HashMap::new();

    for (person, ps) in &state {
        // Skip dead persons
        if let Some(dead_at) = ps.dead_at
            && dead_at <= query_max_key
        {
            continue;
        }

        // Skip persons with no known location
        let loc = match &ps.location {
            Some(l) => l,
            None => continue,
        };

        // Skip stale persons
        if is_stale(ps.last_seen, query_max_key) {
            continue;
        }

        let status = if loc.ad_year == query_max_key {
            "current"
        } else {
            "last_known"
        };

        result.insert(
            person.clone(),
            PersonLocation {
                location: loc.place.clone(),
                role: loc.role.clone(),
                as_of: loc.as_of.clone(),
                status: status.to_string(),
            },
        );
    }

    if result.is_empty() {
        eprintln!("No person-location mappings found for: {raw}");
        return;
    }

    eprintln!(
        "Found {} persons with location data for: {}",
        result.len(),
        raw
    );

    let json = serde_json::to_string_pretty(&result).expect("JSON serialization");
    println!("{json}");
}

#[derive(serde::Serialize)]
struct PersonLocation {
    location: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<String>,
    as_of: String,
    status: String,
}

// ═══════════════════════════════════════════════════════════════════════
//  EXTRACT MODE: full corpus processing → output/*.json
// ═══════════════════════════════════════════════════════════════════════

fn run_extract(root: &Path) {
    eprintln!("Scanning corpus at: {}", root.display());

    // Phase 1: discover all biography files
    let bio_files = scanner::scan_corpus(root);
    eprintln!("Found {} biography/annals files", bio_files.len());

    // Phase 2: parse person info from each file
    let mut persons = Vec::new();
    let mut failed = Vec::new();

    for bio in &bio_files {
        match parser::parse_biography(bio) {
            Some(person) => persons.push(person),
            None => failed.push(bio.path.display().to_string()),
        }
    }

    eprintln!(
        "Parsed {} persons ({} files could not be parsed)",
        persons.len(),
        failed.len()
    );

    // Phase 3: compute reference stats and build summaries
    let summaries: Vec<PersonSummary> = persons.iter().map(PersonSummary::from_person).collect();

    // ── Print statistics ───────────────────────────────────────────
    eprintln!("\n══════════════════════════════════════════");
    eprintln!("  CORPUS STATISTICS");
    eprintln!("══════════════════════════════════════════");

    let mut by_book = std::collections::HashMap::new();
    for p in &persons {
        *by_book.entry(p.source.book.as_chinese()).or_insert(0usize) += 1;
    }
    eprintln!("\nBy book:");
    let mut book_counts: Vec<_> = by_book.iter().collect();
    book_counts.sort_by_key(|(_, c)| std::cmp::Reverse(**c));
    for (book, count) in &book_counts {
        eprintln!("  {book}: {count} persons");
    }

    let mut by_section = std::collections::HashMap::new();
    for p in &persons {
        let sec = match p.source.section {
            Section::BenJi => "本紀",
            Section::LieZhuan => "列傳",
            Section::ZaiJi => "載記",
            Section::Zhi => "志",
            Section::Other => "其他",
        };
        *by_section.entry(sec).or_insert(0usize) += 1;
    }
    eprintln!("\nBy section:");
    for (sec, count) in &by_section {
        eprintln!("  {sec}: {count} persons");
    }

    let (mut emperors, mut officials, mut deposed, mut rulers) = (0usize, 0, 0, 0);
    for p in &persons {
        match &p.kind {
            types::PersonKind::Emperor { .. } => emperors += 1,
            types::PersonKind::Official { .. } => officials += 1,
            types::PersonKind::Deposed { .. } => deposed += 1,
            types::PersonKind::Ruler { .. } => rulers += 1,
        }
    }
    eprintln!("\nBy kind:");
    eprintln!("  Emperor:  {emperors}");
    eprintln!("  Official: {officials}");
    eprintln!("  Ruler:    {rulers}");
    eprintln!("  Deposed:  {deposed}");

    // ── Print failures ─────────────────────────────────────────────
    if !failed.is_empty() {
        eprintln!("\n══════════════════════════════════════════");
        eprintln!("  UNPARSED FILES ({} total)", failed.len());
        eprintln!("══════════════════════════════════════════");
        for f in failed.iter().take(30) {
            eprintln!("  {f}");
        }
        if failed.len() > 30 {
            eprintln!("  ... and {} more", failed.len() - 30);
        }
    }

    // ── Phase 4: In-text person name extraction ────────────────────
    eprintln!("\n══════════════════════════════════════════");
    eprintln!("  IN-TEXT PERSON NAME RECOGNITION");
    eprintln!("══════════════════════════════════════════");

    let name_scanner = intext::InTextScanner::new(&persons);
    let in_text_persons = name_scanner.scan_corpus(&bio_files);

    let total_mentions: usize = in_text_persons.iter().map(|p| p.mention_count).sum();
    let unknown_persons: Vec<_> = in_text_persons
        .iter()
        .filter(|p| !p.has_own_biography)
        .collect();

    eprintln!(
        "\nFound {} unique names with {} total mentions",
        in_text_persons.len(),
        total_mentions
    );
    eprintln!(
        "  Known (have own biography): {}",
        in_text_persons.len() - unknown_persons.len()
    );
    eprintln!("  Unknown (in-text only):     {}", unknown_persons.len());

    eprintln!("\nTop unknown persons (no own biography):");
    for p in unknown_persons.iter().take(20) {
        let files_short: Vec<&str> = p
            .mentioned_in
            .iter()
            .take(3)
            .map(|f| f.rsplit('/').next().unwrap_or(f))
            .collect();
        let patterns: Vec<String> = p
            .pattern_counts
            .iter()
            .map(|(k, v)| format!("{k}×{v}"))
            .collect();
        eprintln!(
            "  {} ({}次, {}) — {}",
            p.name,
            p.mention_count,
            patterns.join(", "),
            files_short.join(", ")
        );
    }

    // ── Phase 5: Event extraction (time + place + person) ───────────
    eprintln!("\n══════════════════════════════════════════");
    eprintln!("  EVENT EXTRACTION");
    eprintln!("══════════════════════════════════════════");

    let event_scanner = event::EventScanner::new(&persons);
    let (events, time_index, event_stats) = event_scanner.scan_corpus(&bio_files);

    // ── Phase 6: Build timeline ─────────────────────────────────────
    let timeline = event::Timeline::from_scopes(&time_index.scopes);

    eprintln!(
        "\nExtracted {} events, {} time scopes, {} time points",
        event_stats.total_events,
        time_index.scopes.len(),
        timeline.total_time_points
    );
    eprintln!("  Appointments: {}", event_stats.appointments);
    eprintln!("  Battles:      {}", event_stats.battles);
    eprintln!("  Deaths:       {}", event_stats.deaths);

    // Era distribution
    let mut era_counts: Vec<_> = event_stats.era_distribution.iter().collect();
    era_counts.sort_by_key(|(_, c)| std::cmp::Reverse(**c));
    eprintln!("\nEra distribution (top 15):");
    for (era, count) in era_counts.iter().take(15) {
        eprintln!("  {era}: {count} events");
    }

    // Timeline summary
    eprintln!("\nTimeline by regime:");
    for regime in &timeline.regimes {
        let era_count = regime.eras.len();
        let year_count: usize = regime.eras.iter().map(|e| e.years.len()).sum();
        eprintln!(
            "  {}: {} eras, {} distinct years",
            regime.regime, era_count, year_count
        );
    }

    // Sample events
    eprintln!("\nSample events (first 10):");
    for e in events.iter().take(10) {
        let time_str = e
            .time
            .as_ref()
            .map(|t| format!("[{}/{}{}年]", t.regime, t.era, t.year))
            .unwrap_or_default();

        let event_str = match &e.kind {
            event::EventKind::Appointment {
                person,
                new_title,
                place,
            } => {
                let place_str = place
                    .as_ref()
                    .map(|p| {
                        if p.is_qiao {
                            format!(" @{}(僑)", p.name)
                        } else {
                            format!(" @{}", p.name)
                        }
                    })
                    .unwrap_or_default();
                format!("任命 {}→{}{}", person, new_title, place_str)
            }
            event::EventKind::Battle {
                person,
                verb,
                target,
                target_place,
            } => {
                let place_str = target_place
                    .as_ref()
                    .map(|p| format!(" @{}", p.name))
                    .unwrap_or_default();
                format!("戰事 {}{}{}{}", person, verb, target, place_str)
            }
            event::EventKind::Death { person, verb } => {
                format!("死亡 {}{}", person, verb)
            }
        };
        eprintln!("  {} {}", time_str, event_str);
    }

    // ── Build frequency maps for high-confidence filtering ─────────
    let mut person_freq: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    let mut location_freq: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    for e in &events {
        *person_freq.entry(e.person_name().to_string()).or_insert(0) += 1;
        for loc_name in e.all_location_names() {
            *location_freq.entry(loc_name.to_string()).or_insert(0) += 1;
        }
    }

    let high_conf_persons: usize = person_freq.values().filter(|&&c| c >= 2).count();
    let high_conf_locations: usize = location_freq.values().filter(|&&c| c >= 2).count();
    eprintln!(
        "\nHigh-confidence (freq >= 2): {} persons, {} locations",
        high_conf_persons, high_conf_locations,
    );

    // ── Write split JSON files ──────────────────────────────────────
    eprintln!("\n══════════════════════════════════════════");
    eprintln!("  WRITING OUTPUT FILES");
    eprintln!("══════════════════════════════════════════\n");

    std::fs::create_dir_all(OUTPUT_DIR).expect("cannot create output/");

    // 1. persons.json — biography summaries + in-text mentions + event person frequencies
    #[derive(serde::Serialize)]
    struct EventPersonEntry {
        name: String,
        event_count: usize,
    }
    #[derive(serde::Serialize)]
    struct PersonsOutput {
        persons: Vec<extract::PersonSummary>,
        in_text_mentions: Vec<intext::InTextPerson>,
        event_persons: Vec<EventPersonEntry>,
    }
    let mut event_persons: Vec<EventPersonEntry> = person_freq
        .iter()
        .map(|(name, &count)| EventPersonEntry {
            name: name.clone(),
            event_count: count,
        })
        .collect();
    event_persons.sort_by(|a, b| b.event_count.cmp(&a.event_count));
    write_json(
        "persons.json",
        &PersonsOutput {
            persons: summaries,
            in_text_mentions: in_text_persons,
            event_persons,
        },
    );

    // 2. locations.json — all raw location extractions, grouped by name
    #[derive(serde::Serialize)]
    struct LocationSource {
        source_file: String,
        byte_offset: usize,
        #[serde(skip_serializing_if = "Option::is_none")]
        time: Option<String>,
    }
    #[derive(serde::Serialize)]
    struct LocationEntry {
        name: String,
        is_qiao: bool,
        #[serde(skip_serializing_if = "Option::is_none")]
        role_suffix: Option<String>,
        event_count: usize,
        sources: Vec<LocationSource>,
    }

    // Collect all location occurrences grouped by name
    let mut loc_map: std::collections::HashMap<String, LocationEntry> =
        std::collections::HashMap::new();
    for e in &events {
        let time_str = e
            .time
            .as_ref()
            .map(|t| format!("{}/{}{}年", t.regime, t.era, t.year));

        // Gather all PlaceRefs from this event
        let mut refs_in_event: Vec<&event::PlaceRef> = e.locations.iter().collect();
        match &e.kind {
            event::EventKind::Appointment { place: Some(p), .. }
            | event::EventKind::Battle {
                target_place: Some(p),
                ..
            } => refs_in_event.push(p),
            _ => {}
        }

        for pr in refs_in_event {
            let entry = loc_map
                .entry(pr.name.clone())
                .or_insert_with(|| LocationEntry {
                    name: pr.name.clone(),
                    is_qiao: pr.is_qiao,
                    role_suffix: pr.role_suffix.clone(),
                    event_count: 0,
                    sources: Vec::new(),
                });
            entry.event_count += 1;
            entry.sources.push(LocationSource {
                source_file: e.source_file.clone(),
                byte_offset: e.byte_offset,
                time: time_str.clone(),
            });
        }
    }
    let mut locations: Vec<LocationEntry> = loc_map.into_values().collect();
    locations.sort_by(|a, b| b.event_count.cmp(&a.event_count));
    write_json("locations.json", &locations);

    // 3. events.json — split into high-confidence and unstructured
    #[derive(serde::Serialize)]
    struct EventsOutput {
        events: Vec<event::Event>,
        unstructured_events: Vec<event::Event>,
    }

    let mut high_confidence = Vec::new();
    let mut unstructured = Vec::new();
    for e in events {
        let person_count = person_freq.get(e.person_name()).copied().unwrap_or(0);
        if person_count >= 2 {
            // Filter locations to only high-confidence
            let mut filtered = e;
            filtered
                .locations
                .retain(|l| location_freq.get(l.name.as_str()).copied().unwrap_or(0) >= 2);
            // Also filter structured place fields
            match &mut filtered.kind {
                event::EventKind::Appointment { place, .. } => {
                    if let Some(p) = place
                        && location_freq.get(p.name.as_str()).copied().unwrap_or(0) < 2
                    {
                        *place = None;
                    }
                }
                event::EventKind::Battle { target_place, .. } => {
                    if let Some(p) = target_place
                        && location_freq.get(p.name.as_str()).copied().unwrap_or(0) < 2
                    {
                        *target_place = None;
                    }
                }
                event::EventKind::Death { .. } => {}
            }
            high_confidence.push(filtered);
        } else {
            unstructured.push(e);
        }
    }

    eprintln!(
        "  events: {} high-confidence, {} unstructured",
        high_confidence.len(),
        unstructured.len(),
    );
    write_json(
        "events.json",
        &EventsOutput {
            events: high_confidence,
            unstructured_events: unstructured,
        },
    );

    // 4. timeline.json — timeline + time_index + stats
    #[derive(serde::Serialize)]
    struct TimelineOutput {
        timeline: event::Timeline,
        time_index: event::TimeIndex,
        stats: event::EventStats,
    }
    write_json(
        "timeline.json",
        &TimelineOutput {
            timeline,
            time_index,
            stats: event_stats,
        },
    );

    eprintln!("\nDone. Query with:");
    eprintln!("  cargo run -- query \"太和三年\"");
    eprintln!("  cargo run -- query \"太和元年-太和六年\"");
    eprintln!("  cargo run -- query \"@東晉\"");
    eprintln!("  cargo run -- timeline");
}
