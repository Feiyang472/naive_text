#![allow(dead_code)]
use serde::Deserialize;
use std::collections::HashMap;

// ── events.json ──────────────────────────────────────────────────────────────

#[derive(Deserialize, Clone, Debug)]
pub struct EventsJson {
    pub high_confidence: Vec<Event>,
    pub unstructured: Vec<Event>,
}

#[derive(Deserialize, Clone, Debug)]
#[serde(tag = "type")]
pub enum Event {
    Appointment(EventData),
    Promotion(EventData),
    Accession(EventData),
    Battle(EventData),
    Death(EventData),
}

impl Event {
    pub fn data(&self) -> &EventData {
        match self {
            Event::Appointment(d)
            | Event::Promotion(d)
            | Event::Accession(d)
            | Event::Battle(d)
            | Event::Death(d) => d,
        }
    }

    pub fn kind_str(&self) -> &'static str {
        match self {
            Event::Appointment(_) => "Appointment",
            Event::Promotion(_) => "Promotion",
            Event::Accession(_) => "Accession",
            Event::Battle(_) => "Battle",
            Event::Death(_) => "Death",
        }
    }

    pub fn kind_zh(&self) -> &'static str {
        match self {
            Event::Appointment(_) => "任命",
            Event::Promotion(_) => "晋升",
            Event::Accession(_) => "即位",
            Event::Battle(_) => "战役",
            Event::Death(_) => "薨卒",
        }
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct EventData {
    pub person_name: String,
    #[serde(default)]
    pub time: Option<String>,
    #[serde(default)]
    pub place: Option<String>,
    pub context: String,
    pub source_file: String,
    pub byte_offset: usize,
    #[serde(default)]
    pub ad_year: Option<i32>,
}

// ── timeline.json ─────────────────────────────────────────────────────────────

#[derive(Deserialize, Clone, Debug)]
pub struct TimelineJson {
    pub timeline: TimelineData,
    pub stats: EventStats,
}

#[derive(Deserialize, Clone, Debug)]
pub struct TimelineData {
    pub regimes: Vec<RegimeTimeline>,
    pub total_time_points: usize,
}

#[derive(Deserialize, Clone, Debug)]
pub struct RegimeTimeline {
    pub regime: String,
    pub eras: Vec<EraTimeline>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct EraTimeline {
    pub era: String,
    pub years: Vec<EraTimePoint>,
}

impl EraTimeline {
    pub fn total_occurrences(&self) -> usize {
        self.years.iter().map(|y| y.occurrence_count).sum()
    }
}

#[derive(Deserialize, Clone, Debug)]
pub struct EraTimePoint {
    pub era: String,
    pub year: u32,
    pub occurrence_count: usize,
    #[serde(default)]
    pub files: Vec<String>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct EventStats {
    pub total_events: usize,
    pub appointments: usize,
    pub promotions: usize,
    pub accessions: usize,
    pub battles: usize,
    pub deaths: usize,
    #[serde(default)]
    pub unique_time_refs: usize,
    #[serde(default)]
    pub unique_places: usize,
    #[serde(default)]
    pub top_places: Vec<(String, usize)>,
    #[serde(default)]
    pub era_distribution: HashMap<String, usize>,
}
