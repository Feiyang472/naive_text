use std::collections::HashMap;
use std::fs;

use crate::types::*;

/// Statistics about how a person is referred to in their own biography.
#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct RefStats {
    /// How many times each alias appears in the text
    pub alias_counts: HashMap<String, usize>,
    /// Total lines in the biography
    pub total_lines: usize,
}

/// Count how often each alias of a person appears in their biography text.
pub fn count_refs_in_biography(person: &Person) -> RefStats {
    let content = match fs::read_to_string(&person.source.file_path) {
        Ok(c) => c,
        Err(_) => return RefStats::default(),
    };

    let lines: Vec<&str> = content.lines().collect();
    let total_lines = lines.len();

    let mut alias_counts = HashMap::new();

    for alias in &person.aliases {
        if alias.is_empty() {
            continue;
        }
        let count = content.matches(alias).count();
        if count > 0 {
            alias_counts.insert(alias.clone(), count);
        }
    }

    RefStats {
        alias_counts,
        total_lines,
    }
}

/// A summary of a person and how they're referred to.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct PersonSummary {
    pub display_name: String,
    pub book: String,
    pub section: String,
    pub kind: String,
    pub aliases: Vec<String>,
    pub ref_stats: RefStats,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub courtesy_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
    pub file: String,
}

impl PersonSummary {
    pub fn from_person(person: &Person) -> Self {
        let ref_stats = count_refs_in_biography(person);

        let (kind, courtesy_name, origin) = match &person.kind {
            PersonKind::Emperor {
                courtesy_name,
                ..
            } => {
                let cn = match courtesy_name {
                    CourtesyName::Recorded(c) => Some(c.clone()),
                    CourtesyName::NotRecorded => None,
                };
                ("Emperor".to_string(), cn, None)
            }
            PersonKind::Official {
                courtesy_name,
                origin,
                ..
            } => {
                let cn = match courtesy_name {
                    CourtesyName::Recorded(c) => Some(c.clone()),
                    CourtesyName::NotRecorded => None,
                };
                ("Official".to_string(), cn, origin.clone())
            }
            PersonKind::Deposed {
                courtesy_name,
                ..
            } => {
                let cn = match courtesy_name {
                    CourtesyName::Recorded(c) => Some(c.clone()),
                    CourtesyName::NotRecorded => None,
                };
                ("Deposed".to_string(), cn, None)
            }
            PersonKind::Ruler {
                courtesy_name,
                lineage,
                ..
            } => {
                let cn = match courtesy_name {
                    CourtesyName::Recorded(c) => Some(c.clone()),
                    CourtesyName::NotRecorded => None,
                };
                ("Ruler".to_string(), cn, lineage.clone())
            }
        };

        let section = match person.source.section {
            Section::BenJi => "本紀",
            Section::LieZhuan => "列傳",
            Section::ZaiJi => "載記",
            Section::Zhi => "志",
            Section::Other => "其他",
        };

        PersonSummary {
            display_name: person.display_name(),
            book: person.source.book.as_chinese().to_string(),
            section: section.to_string(),
            kind,
            aliases: person.aliases.clone(),
            ref_stats,
            courtesy_name,
            origin,
            file: person.source.file_path.display().to_string(),
        }
    }
}
