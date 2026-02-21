#![allow(dead_code)]
use serde::{Deserialize, Serialize};

// ── Place reference ──────────────────────────────────────────────────────

/// A place name extracted from appointment or military context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlaceRef {
    pub name: String,
    pub is_qiao: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role_suffix: Option<String>, // 刺史, 太守, etc.
}

// ── Time reference ───────────────────────────────────────────────────────

/// A time reference extracted from text, scoped to a regime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeRef {
    pub era: String,
    pub regime: String,
    pub year: u8,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub month: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub day_ganzhi: Option<String>,
    /// Raw matched text
    pub raw: String,
    /// Byte offset where this time reference appears in the source file
    pub byte_offset: usize,
}

// ── Event types ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum EventKind {
    /// 以X為Y — person appointed to a position (possibly at a place)
    Appointment {
        person: String,
        new_title: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        place: Option<PlaceRef>,
    },
    /// 拜/除/遷/轉/授/徵/封 X 為 Y — official transfer, promotion, or enfeoffment
    Promotion {
        person: String,
        /// The appointing/transferring verb (拜/除/遷/轉/授/徵/封)
        verb: String,
        new_title: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub kind: EventKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
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

    /// Get Chinese description of the event kind (for UI display).
    pub fn kind_zh(&self) -> &'static str {
        match &self.kind {
            EventKind::Appointment { .. } => "任命",
            EventKind::Promotion { .. } => "晋升",
            EventKind::Accession { .. } => "即位",
            EventKind::Battle { .. } => "战役",
            EventKind::Death { .. } => "薨卒",
        }
    }
}

// ── JSON output format ─────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct EventsOutput {
    pub events: Vec<Event>,
    pub unstructured_events: Vec<Event>,
}
