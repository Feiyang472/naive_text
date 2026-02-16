use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::types::{Book, Section, Source};

/// A biography file discovered in the corpus.
#[derive(Debug)]
pub struct BiographyFile {
    pub source: Source,
    pub path: PathBuf,
}

/// Scan the corpus root and discover all biography/annals text files.
///
/// Expected directory layout:
///   {root}/{書名}/{NN_section}/{NN_卷名}/{NN_人名.txt}
///
/// We skip files named "目录.txt", "史論.txt", "史評.txt", "論.txt",
/// "評.txt", "贊.txt", "序.txt", "注.txt" – those don't contain
/// biography openings.
pub fn scan_corpus(root: &Path) -> Vec<BiographyFile> {
    let skip_names: &[&str] = &[
        "目录", "史論", "史評", "論", "評", "贊", "評贊", "序", "注", "正文", "附錄",
    ];

    let mut results = Vec::new();

    for book_entry in std::fs::read_dir(root).into_iter().flatten() {
        let book_entry = match book_entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        let book_dir = book_entry.path();
        if !book_dir.is_dir() {
            continue;
        }

        let book_name = book_dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let book = match Book::from_dir_name(book_name) {
            Some(b) => b,
            None => continue, // skip Cargo.toml, src/, etc.
        };

        // Walk into section directories
        for section_entry in std::fs::read_dir(&book_dir).into_iter().flatten() {
            let section_entry = match section_entry {
                Ok(e) => e,
                Err(_) => continue,
            };
            let section_dir = section_entry.path();
            if !section_dir.is_dir() {
                continue;
            }

            let section_name = section_dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            let section = Section::from_dir_name(section_name);

            // Walk into juan (volume) directories
            for juan_entry in std::fs::read_dir(&section_dir).into_iter().flatten() {
                let juan_entry = match juan_entry {
                    Ok(e) => e,
                    Err(_) => continue,
                };
                let juan_dir = juan_entry.path();
                if !juan_dir.is_dir() {
                    continue;
                }

                let juan_name = juan_dir
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();

                // Scan .txt files inside the juan directory
                for file_entry in WalkDir::new(&juan_dir)
                    .min_depth(1)
                    .max_depth(1)
                    .into_iter()
                    .filter_map(|e| e.ok())
                {
                    let path = file_entry.path().to_path_buf();
                    if path.extension().and_then(|e| e.to_str()) != Some("txt") {
                        continue;
                    }

                    let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");

                    // Strip leading numeric prefix (e.g. "02_褚淵" → "褚淵")
                    let clean_stem = strip_numeric_prefix(stem);

                    // Skip non-biography files, but keep "目录" in 本紀/載記
                    // where it often contains the actual biography text
                    if skip_names.contains(&clean_stem) {
                        let keep = clean_stem == "目录"
                            && matches!(section, Section::BenJi | Section::ZaiJi);
                        if !keep {
                            continue;
                        }
                    }

                    // Skip year-based files (e.g. "永明五年", "建元元年")
                    if is_year_file(clean_stem) {
                        continue;
                    }

                    results.push(BiographyFile {
                        source: Source {
                            book,
                            section,
                            juan: juan_name.clone(),
                            file_path: path.clone(),
                        },
                        path,
                    });
                }
            }
        }
    }

    results
}

/// Strip leading "NN_" prefix from filenames.
fn strip_numeric_prefix(s: &str) -> &str {
    if let Some(idx) = s.find('_') {
        let prefix = &s[..idx];
        if prefix.chars().all(|c| c.is_ascii_digit()) {
            return &s[idx + 1..];
        }
    }
    s
}

/// Check if a filename looks like a year entry (e.g. "永明五年", "太康元年").
fn is_year_file(name: &str) -> bool {
    name.ends_with("年") || name.ends_with("年餘")
}
