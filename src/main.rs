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

use extract::PersonSummary;
use types::Section;

const OUTPUT_PATH: &str = "output/corpus.json";

fn main() {
    let args: Vec<String> = std::env::args().collect();

    // --query MODE: read cached JSON, return matching time scopes
    if args.len() >= 3 && args[1] == "--query" {
        run_query(&args[2..]);
        return;
    }

    // Default: full extraction
    let root = args
        .get(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(".").to_path_buf());

    run_extract(&root);
}

// ═══════════════════════════════════════════════════════════════════════
//  QUERY MODE: read output/corpus.json and return matching time scopes
// ═══════════════════════════════════════════════════════════════════════

#[derive(serde::Deserialize, serde::Serialize)]
struct Output {
    persons: Vec<extract::PersonSummary>,
    in_text_mentions: Vec<intext::InTextPerson>,
    events: Vec<event::Event>,
    time_index: event::TimeIndex,
    event_stats: event::EventStats,
}

fn run_query(query_args: &[String]) {
    let raw = query_args.join(" ");

    // Read cached JSON
    let json = match std::fs::read_to_string(OUTPUT_PATH) {
        Ok(j) => j,
        Err(e) => {
            eprintln!("Cannot read {OUTPUT_PATH}: {e}");
            eprintln!("Run extraction first (without --query) to generate the index.");
            std::process::exit(1);
        }
    };

    let output: Output = match serde_json::from_str(&json) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("Cannot parse {OUTPUT_PATH}: {e}");
            eprintln!("The JSON may be from an older format. Re-run extraction.");
            std::process::exit(1);
        }
    };

    // Parse query: "{era}" or "{era}{year}年" or "{era}{number}"
    // e.g. "太和", "太和三年", "太和3"
    let (era, year) = parse_time_query(&raw);

    let matches = output.time_index.query(&era, year);

    if matches.is_empty() {
        eprintln!("No time scopes found for: {raw}");
        eprintln!("  parsed as: era={era}, year={year:?}");
        // Show available eras
        let mut eras: Vec<&str> = output
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

    eprintln!(
        "Found {} time scope(s) for: era={}, year={:?}",
        matches.len(),
        era,
        year
    );

    // Output matching scopes as JSON to stdout
    #[derive(serde::Serialize)]
    struct QueryResult<'a> {
        query_era: &'a str,
        query_year: Option<u8>,
        match_count: usize,
        scopes: Vec<&'a event::TimeScope>,
    }

    let result = QueryResult {
        query_era: &era,
        query_year: year,
        match_count: matches.len(),
        scopes: matches,
    };

    let json = serde_json::to_string_pretty(&result).expect("JSON serialization");
    println!("{json}");
}

/// Parse a time query like "太和三年", "太和3", "太和" into (era, Option<year>).
fn parse_time_query(raw: &str) -> (String, Option<u8>) {
    let raw = raw.trim().trim_end_matches('年');

    // Try to split off a trailing Arabic number: "太和3" → ("太和", Some(3))
    if let Some(idx) = raw.rfind(|c: char| !c.is_ascii_digit()) {
        let after = &raw[idx + raw[idx..].chars().next().unwrap().len_utf8()..];
        if !after.is_empty() {
            if let Ok(y) = after.parse::<u8>() {
                return (raw[..idx + raw[idx..].chars().next().unwrap().len_utf8()].to_string(), Some(y));
            }
        }
    }

    // Try Chinese number suffix: "太和三" → ("太和", Some(3))
    // Check last 1-3 chars for a Chinese number
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
//  EXTRACT MODE: full corpus processing → output/corpus.json
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

    // Count by book
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

    // Count by section
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

    // Count by person kind
    let mut emperors = 0usize;
    let mut officials = 0usize;
    let mut deposed = 0usize;
    let mut rulers = 0usize;
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

    // Show some examples
    eprintln!("\n══════════════════════════════════════════");
    eprintln!("  SAMPLE PERSONS");
    eprintln!("══════════════════════════════════════════");

    for s in summaries.iter().take(20) {
        eprintln!(
            "\n  {} ({} / {})",
            s.display_name, s.book, s.section
        );
        eprintln!("    Kind: {}", s.kind);
        if let Some(ref c) = s.courtesy_name {
            eprintln!("    Courtesy name: {c}");
        }
        if let Some(ref o) = s.origin {
            eprintln!("    Origin: {o}");
        }
        eprintln!("    Aliases: {:?}", s.aliases);
        if !s.ref_stats.alias_counts.is_empty() {
            let mut counts: Vec<_> = s.ref_stats.alias_counts.iter().collect();
            counts.sort_by_key(|(_, c)| std::cmp::Reverse(**c));
            let top: Vec<String> = counts
                .iter()
                .take(5)
                .map(|(name, count)| format!("{}×{}", name, count))
                .collect();
            eprintln!("    Refs in text: {}", top.join(", "));
        }
    }

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
    let unknown_persons: Vec<_> = in_text_persons.iter().filter(|p| !p.has_own_biography).collect();

    eprintln!(
        "\nFound {} unique names with {} total mentions",
        in_text_persons.len(),
        total_mentions
    );
    eprintln!(
        "  Known (have own biography): {}",
        in_text_persons.len() - unknown_persons.len()
    );
    eprintln!(
        "  Unknown (in-text only):     {}",
        unknown_persons.len()
    );

    // Show top unknown persons
    eprintln!("\nTop unknown persons (no own biography):");
    for p in unknown_persons.iter().take(30) {
        let files_short: Vec<&str> = p
            .mentioned_in
            .iter()
            .take(3)
            .map(|f| {
                f.rsplit('/')
                    .next()
                    .unwrap_or(f)
            })
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

    // Show top known persons by cross-reference count
    let known_persons_list: Vec<_> = in_text_persons.iter().filter(|p| p.has_own_biography).collect();
    eprintln!("\nTop known persons (most cross-referenced):");
    for p in known_persons_list.iter().take(20) {
        eprintln!(
            "  {} — {}次 across {} files",
            p.name,
            p.mention_count,
            p.mentioned_in.len()
        );
    }

    // ── Phase 5: Event extraction (time + place + person) ───────────
    eprintln!("\n══════════════════════════════════════════");
    eprintln!("  EVENT EXTRACTION");
    eprintln!("══════════════════════════════════════════");

    let event_scanner = event::EventScanner::new(&persons);
    let (events, time_index, event_stats) = event_scanner.scan_corpus(&bio_files);

    eprintln!(
        "\nExtracted {} events, {} time scopes",
        event_stats.total_events,
        time_index.scopes.len()
    );
    eprintln!("  Appointments: {}", event_stats.appointments);
    eprintln!("  Battles:      {}", event_stats.battles);
    eprintln!("  Deaths:       {}", event_stats.deaths);
    eprintln!(
        "  Unique time refs: {}",
        event_stats.unique_time_refs
    );

    // Era distribution
    let mut era_counts: Vec<_> = event_stats.era_distribution.iter().collect();
    era_counts.sort_by_key(|(_, c)| std::cmp::Reverse(**c));
    eprintln!("\nEra distribution (top 15):");
    for (era, count) in era_counts.iter().take(15) {
        eprintln!("  {era}: {count} events");
    }

    // Top places
    eprintln!("\nTop places:");
    for (place, count) in event_stats.top_places.iter().take(20) {
        eprintln!("  {place}: {count} events");
    }

    // Sample events
    eprintln!("\nSample events (first 15):");
    for e in events.iter().take(15) {
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
            } => format!("戰事 {}{}{}", person, verb, target),
            event::EventKind::Death { person, verb } => {
                format!("死亡 {}{}", person, verb)
            }
        };
        eprintln!("  {} {}", time_str, event_str);
    }

    // ── Write JSON to output/corpus.json ────────────────────────────
    let output = Output {
        persons: summaries,
        in_text_mentions: in_text_persons,
        events,
        time_index,
        event_stats,
    };

    // Ensure output directory exists
    std::fs::create_dir_all("output").expect("cannot create output/");

    let json = serde_json::to_string_pretty(&output).expect("JSON serialization failed");
    std::fs::write(OUTPUT_PATH, &json).expect("cannot write output/corpus.json");
    eprintln!("\n✓ Wrote {OUTPUT_PATH} ({} bytes)", json.len());
}
