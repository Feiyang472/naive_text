#![allow(dead_code)]
use serde::Deserialize;
use std::collections::HashMap;

// Re-export shared types from person_types
pub use person_types::{Event, EventKind};

// ── events.json ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Deserialize)]
pub struct EventsJson {
    pub events: Vec<Event>,
    pub unstructured_events: Vec<Event>,
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
