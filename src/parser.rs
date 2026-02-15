use regex::Regex;
use std::fs;
use std::sync::LazyLock;

use crate::scanner::BiographyFile;
use crate::surname::split_name;
use crate::types::*;

// ── Regex patterns ─────────────────────────────────────────────────
//
// Real data examples:
//   Official:
//     褚淵字彥回，河南陽翟人也。
//     韓秀，字白虎，昌黎人也。
//     裴邃字淵明，河東聞喜人，
//     柳世隆字彥緒，河東解人也。
//
//   Emperor (本紀):
//     宣皇帝諱懿，字仲達，河內溫縣孝敬里人，姓司馬氏。
//     高祖武皇帝，諱衍，字叔達，小字練兒，南蘭陵中都里人
//     高祖武皇帝諱霸先，字興國，小字法生，吳興長城下若里人
//     廢帝諱昱，字德融，小字慧震，明帝長子也。
//     世祖武皇帝諱賾，字宣遠，太祖長子也。

// Pattern 1: Official biography opening
// {FullName}[，]字{Courtesy}[，]{Origin}人[也]。
static RE_OFFICIAL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(?P<name>[^\s，。、字]{2,4})[，,]?字(?P<courtesy>[^\s，。]{1,3})[，,](?P<origin>[^\s，。人]+)人"
    ).unwrap()
});

// Pattern 1b: Official without courtesy name
// {FullName}，{Origin}人也。
static RE_OFFICIAL_NO_COURTESY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(?P<name>[^\s，。、字]{2,4})[，,](?P<origin>[^\s，。人字]+)人也"
    ).unwrap()
});

// Pattern 2a: Emperor with temple name
// {TempleName}{Posthumous}皇帝[，]諱{Given}，字{Courtesy}[，小字{Childhood}]
// Temple names are always exactly 2 chars: 高祖, 太宗, 世祖, etc.
// Posthumous titles are 1-2 chars: 武, 宣, 孝武, 簡文, etc.
static RE_EMPEROR_TEMPLE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(?P<temple>[^\s，。諱]{2})(?P<posthumous>[^\s，。諱]{1,2})皇帝[，,]?諱(?P<given>[^\s，。]{1,2})[，,]字(?P<courtesy>[^\s，。]{1,3})(?:[，,]小字(?P<childhood>[^\s，。]{1,3}))?"
    ).unwrap()
});

// Pattern 2b: Emperor without temple name (shorter prefix)
// {Posthumous}皇帝諱{Given}，字{Courtesy}
static RE_EMPEROR_SHORT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(?P<posthumous>[^\s，。諱]{1,4})皇帝諱(?P<given>[^\s，。]{1,2})[，,]字(?P<courtesy>[^\s，。]{1,3})(?:[，,]小字(?P<childhood>[^\s，。]{1,3}))?"
    ).unwrap()
});

// Pattern 2c: Deposed ruler / prince
// {Title}諱{Given}，字{Courtesy}[，小字{Childhood}]
static RE_DEPOSED: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(?P<title>[^\s，。諱]{2,4})諱(?P<given>[^\s，。]{1,2})[，,]字(?P<courtesy>[^\s，。]{1,3})(?:[，,]小字(?P<childhood>[^\s，。]{1,3}))?"
    ).unwrap()
});

// Pattern for surname extraction from "姓X氏"
static RE_SURNAME: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"姓(?P<surname>[^\s，。氏]+)氏").unwrap()
});

/// Try to parse a person from a biography file.
pub fn parse_biography(bio: &BiographyFile) -> Option<Person> {
    let content = fs::read_to_string(&bio.path).ok()?;
    let source = bio.source.clone();

    // For 本紀/載記, the person intro may not be on line 1
    // (some files have headers like "武帝上\n梁書卷第一\n..." first).
    // Try each of the first 10 lines.
    let lines_to_try: Vec<&str> = content
        .lines()
        .take(10)
        .map(|l| l.trim())
        .filter(|l| !l.is_empty())
        .collect();

    if lines_to_try.is_empty() {
        return None;
    }

    for line in &lines_to_try {
        // Try emperor patterns first (for 本紀 and 載記)
        if source.section == Section::BenJi || source.section == Section::ZaiJi {
            if let Some(p) = try_parse_emperor(line, &content, &source) {
                return Some(p);
            }
        }

        // Try official pattern (most common)
        if let Some(p) = try_parse_official(line, &source) {
            return Some(p);
        }

        // Try emperor patterns even in 列傳
        if source.section != Section::BenJi {
            if let Some(p) = try_parse_emperor(line, &content, &source) {
                return Some(p);
            }
        }
    }

    None
}

fn try_parse_official(line: &str, source: &Source) -> Option<Person> {
    if let Some(caps) = RE_OFFICIAL.captures(line) {
        let full_name = caps.name("name")?.as_str();
        let courtesy = caps.name("courtesy")?.as_str();
        let origin = caps.name("origin").map(|m| m.as_str().to_string());

        let (surname, given_name) = split_name(full_name)?;

        let mut person = Person {
            kind: PersonKind::Official {
                surname,
                given_name,
                courtesy_name: CourtesyName::Recorded(courtesy.to_string()),
                origin,
            },
            source: source.clone(),
            aliases: Vec::new(),
        };
        person.compute_aliases();
        return Some(person);
    }

    // Try without courtesy name
    if let Some(caps) = RE_OFFICIAL_NO_COURTESY.captures(line) {
        let full_name = caps.name("name")?.as_str();
        let origin = caps.name("origin").map(|m| m.as_str().to_string());

        let (surname, given_name) = split_name(full_name)?;

        let mut person = Person {
            kind: PersonKind::Official {
                surname,
                given_name,
                courtesy_name: CourtesyName::NotRecorded,
                origin,
            },
            source: source.clone(),
            aliases: Vec::new(),
        };
        person.compute_aliases();
        return Some(person);
    }

    None
}

fn try_parse_emperor(line: &str, full_content: &str, source: &Source) -> Option<Person> {
    // Pattern 2a: with temple name (高祖武皇帝)
    if let Some(caps) = RE_EMPEROR_TEMPLE.captures(line) {
        let temple = caps.name("temple")?.as_str().to_string();
        let posthumous = caps.name("posthumous")?.as_str().to_string();
        let given = caps.name("given")?.as_str().to_string();
        let courtesy = caps
            .name("courtesy")
            .map(|m| CourtesyName::Recorded(m.as_str().to_string()))
            .unwrap_or(CourtesyName::NotRecorded);
        let childhood = caps
            .name("childhood")
            .map(|m| ChildhoodName::Recorded(m.as_str().to_string()))
            .unwrap_or(ChildhoodName::NotRecorded);

        // Try to find surname from "姓X氏" in the text
        let surname = RE_SURNAME
            .captures(full_content)
            .and_then(|c| c.name("surname"))
            .map(|m| m.as_str().to_string());

        let mut person = Person {
            kind: PersonKind::Emperor {
                temple_name: Some(temple),
                posthumous_title: format!("{posthumous}皇帝"),
                given_name: given,
                surname,
                courtesy_name: courtesy,
                childhood_name: childhood,
            },
            source: source.clone(),
            aliases: Vec::new(),
        };
        person.compute_aliases();
        return Some(person);
    }

    // Pattern 2b: without temple name (宣皇帝諱懿)
    if let Some(caps) = RE_EMPEROR_SHORT.captures(line) {
        let posthumous = caps.name("posthumous")?.as_str().to_string();
        let given = caps.name("given")?.as_str().to_string();
        let courtesy = caps
            .name("courtesy")
            .map(|m| CourtesyName::Recorded(m.as_str().to_string()))
            .unwrap_or(CourtesyName::NotRecorded);
        let childhood = caps
            .name("childhood")
            .map(|m| ChildhoodName::Recorded(m.as_str().to_string()))
            .unwrap_or(ChildhoodName::NotRecorded);

        let surname = RE_SURNAME
            .captures(full_content)
            .and_then(|c| c.name("surname"))
            .map(|m| m.as_str().to_string());

        // Try extracting temple name from juan directory name
        let temple_name = extract_temple_from_juan(&source.juan);

        let mut person = Person {
            kind: PersonKind::Emperor {
                temple_name,
                posthumous_title: format!("{posthumous}皇帝"),
                given_name: given,
                surname,
                courtesy_name: courtesy,
                childhood_name: childhood,
            },
            source: source.clone(),
            aliases: Vec::new(),
        };
        person.compute_aliases();
        return Some(person);
    }

    // Pattern 2c: deposed ruler
    if let Some(caps) = RE_DEPOSED.captures(line) {
        let title = caps.name("title")?.as_str().to_string();
        let given = caps.name("given")?.as_str().to_string();
        let courtesy = caps
            .name("courtesy")
            .map(|m| CourtesyName::Recorded(m.as_str().to_string()))
            .unwrap_or(CourtesyName::NotRecorded);
        let childhood = caps
            .name("childhood")
            .map(|m| ChildhoodName::Recorded(m.as_str().to_string()))
            .unwrap_or(ChildhoodName::NotRecorded);

        let mut person = Person {
            kind: PersonKind::Deposed {
                title,
                given_name: given,
                courtesy_name: courtesy,
                childhood_name: childhood,
            },
            source: source.clone(),
            aliases: Vec::new(),
        };
        person.compute_aliases();
        return Some(person);
    }

    None
}

/// Try to extract temple name from the juan directory name.
/// e.g. "00_帝紀第一　高祖宣帝" → Some("高祖")
fn extract_temple_from_juan(juan: &str) -> Option<String> {
    // Common temple names
    let temple_names = [
        "高祖", "太祖", "世祖", "太宗", "世宗", "高宗", "中宗", "肅祖",
        "顯宗", "孝宗",
    ];
    for &tn in &temple_names {
        if juan.contains(tn) {
            return Some(tn.to_string());
        }
    }
    None
}
