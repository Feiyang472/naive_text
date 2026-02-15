mod extract;
mod intext;
mod parser;
mod scanner;
mod surname;
mod titles;
mod types;

use std::path::Path;

use extract::PersonSummary;
use types::Section;

fn main() {
    let root = Path::new(".");

    // Check if a corpus root was passed as argument
    let root = std::env::args()
        .nth(1)
        .map(|s| std::path::PathBuf::from(s))
        .unwrap_or_else(|| root.to_path_buf());

    eprintln!("Scanning corpus at: {}", root.display());

    // Phase 1: discover all biography files
    let bio_files = scanner::scan_corpus(&root);
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

    let scanner = intext::InTextScanner::new(&persons);
    let in_text_persons = scanner.scan_corpus(&bio_files);

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

    // ── JSON output to stdout ──────────────────────────────────────
    #[derive(serde::Serialize)]
    struct Output {
        persons: Vec<extract::PersonSummary>,
        in_text_mentions: Vec<intext::InTextPerson>,
    }

    let output = Output {
        persons: summaries,
        in_text_mentions: in_text_persons,
    };

    let json = serde_json::to_string_pretty(&output).expect("JSON serialization failed");
    println!("{json}");
}
