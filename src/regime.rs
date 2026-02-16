//! Static dictionary of regimes (政權) and their era names (年號).
//!
//! This module provides the mapping needed to disambiguate era names:
//! the same era name (e.g. 太和) can belong to different regimes,
//! and we scope each usage to a specific regime based on which book
//! (晉書/宋書/etc.) the text comes from.

use serde::Serialize;

use crate::types::Book;

// ── Regime ───────────────────────────────────────────────────────────

/// A political regime / dynasty in the Six Dynasties period.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum Regime {
    // ── Unified / major ──
    WesternJin,  // 西晉
    EasternJin,  // 東晉
    LiuSong,     // 劉宋
    SouthernQi,  // 南齊
    Liang,       // 梁
    Chen,        // 陳
    NorthernWei, // 北魏
    // ── Sixteen Kingdoms (十六國) ──
    HanZhao,     // 漢趙 (前趙)
    LaterZhao,   // 後趙
    ChengHan,    // 成漢
    FormerLiang, // 前涼
    FormerYan,   // 前燕
    FormerQin,   // 前秦
    LaterQin,    // 後秦
    LaterYan,    // 後燕
    WesternQin,  // 西秦
    LaterLiang,  // 後涼
    SouthernLiang, // 南涼
    SouthernYan, // 南燕
    WesternLiang, // 西涼
    NorthernLiang, // 北涼
    XiaState,    // 夏 (赫連夏)
    NorthernYan, // 北燕
}

impl Regime {
    pub fn as_chinese(&self) -> &'static str {
        match self {
            Self::WesternJin => "西晉",
            Self::EasternJin => "東晉",
            Self::LiuSong => "劉宋",
            Self::SouthernQi => "南齊",
            Self::Liang => "梁",
            Self::Chen => "陳",
            Self::NorthernWei => "北魏",
            Self::HanZhao => "漢趙",
            Self::LaterZhao => "後趙",
            Self::ChengHan => "成漢",
            Self::FormerLiang => "前涼",
            Self::FormerYan => "前燕",
            Self::FormerQin => "前秦",
            Self::LaterQin => "後秦",
            Self::LaterYan => "後燕",
            Self::WesternQin => "西秦",
            Self::LaterLiang => "後涼",
            Self::SouthernLiang => "南涼",
            Self::SouthernYan => "南燕",
            Self::WesternLiang => "西涼",
            Self::NorthernLiang => "北涼",
            Self::XiaState => "夏",
            Self::NorthernYan => "北燕",
        }
    }
}

// ── Era name entry ───────────────────────────────────────────────────

/// A single era name belonging to one regime.
pub struct EraEntry {
    pub name: &'static str,
    pub regime: Regime,
}

/// Master list of era names for the Six Dynasties period.
/// Ordered by regime, then chronologically within each regime.
/// This is the disambiguation table: given an era name + the source Book,
/// we can resolve which regime it belongs to.
pub static ERA_NAMES: &[EraEntry] = &[
    // ── 西晉 ──
    EraEntry { name: "泰始", regime: Regime::WesternJin },
    EraEntry { name: "咸寧", regime: Regime::WesternJin },
    EraEntry { name: "太康", regime: Regime::WesternJin },
    EraEntry { name: "太熙", regime: Regime::WesternJin },
    EraEntry { name: "永熙", regime: Regime::WesternJin },
    EraEntry { name: "永平", regime: Regime::WesternJin },
    EraEntry { name: "元康", regime: Regime::WesternJin },
    EraEntry { name: "永康", regime: Regime::WesternJin },
    EraEntry { name: "永寧", regime: Regime::WesternJin },
    EraEntry { name: "太安", regime: Regime::WesternJin },
    EraEntry { name: "永安", regime: Regime::WesternJin },
    EraEntry { name: "建武", regime: Regime::WesternJin },
    EraEntry { name: "永興", regime: Regime::WesternJin },
    EraEntry { name: "光熙", regime: Regime::WesternJin },
    EraEntry { name: "永嘉", regime: Regime::WesternJin },
    EraEntry { name: "建興", regime: Regime::WesternJin },
    // ── 東晉 ──
    EraEntry { name: "建武", regime: Regime::EasternJin },
    EraEntry { name: "大興", regime: Regime::EasternJin },
    EraEntry { name: "永昌", regime: Regime::EasternJin },
    EraEntry { name: "太寧", regime: Regime::EasternJin },
    EraEntry { name: "咸和", regime: Regime::EasternJin },
    EraEntry { name: "咸康", regime: Regime::EasternJin },
    EraEntry { name: "建元", regime: Regime::EasternJin },
    EraEntry { name: "永和", regime: Regime::EasternJin },
    EraEntry { name: "升平", regime: Regime::EasternJin },
    EraEntry { name: "隆和", regime: Regime::EasternJin },
    EraEntry { name: "興寧", regime: Regime::EasternJin },
    EraEntry { name: "太和", regime: Regime::EasternJin },
    EraEntry { name: "咸安", regime: Regime::EasternJin },
    EraEntry { name: "寧康", regime: Regime::EasternJin },
    EraEntry { name: "太元", regime: Regime::EasternJin },
    EraEntry { name: "隆安", regime: Regime::EasternJin },
    EraEntry { name: "元興", regime: Regime::EasternJin },
    EraEntry { name: "義熙", regime: Regime::EasternJin },
    // ── 劉宋 ──
    EraEntry { name: "永初", regime: Regime::LiuSong },
    EraEntry { name: "景平", regime: Regime::LiuSong },
    EraEntry { name: "元嘉", regime: Regime::LiuSong },
    EraEntry { name: "孝建", regime: Regime::LiuSong },
    EraEntry { name: "大明", regime: Regime::LiuSong },
    EraEntry { name: "永光", regime: Regime::LiuSong },
    EraEntry { name: "景和", regime: Regime::LiuSong },
    EraEntry { name: "泰始", regime: Regime::LiuSong },
    EraEntry { name: "泰豫", regime: Regime::LiuSong },
    EraEntry { name: "元徽", regime: Regime::LiuSong },
    EraEntry { name: "昇明", regime: Regime::LiuSong },
    // ── 南齊 ──
    EraEntry { name: "建元", regime: Regime::SouthernQi },
    EraEntry { name: "永明", regime: Regime::SouthernQi },
    EraEntry { name: "隆昌", regime: Regime::SouthernQi },
    EraEntry { name: "延興", regime: Regime::SouthernQi },
    EraEntry { name: "建武", regime: Regime::SouthernQi },
    EraEntry { name: "永泰", regime: Regime::SouthernQi },
    EraEntry { name: "永元", regime: Regime::SouthernQi },
    EraEntry { name: "中興", regime: Regime::SouthernQi },
    // ── 梁 ──
    EraEntry { name: "天監", regime: Regime::Liang },
    EraEntry { name: "普通", regime: Regime::Liang },
    EraEntry { name: "大通", regime: Regime::Liang },
    EraEntry { name: "中大通", regime: Regime::Liang },
    EraEntry { name: "大同", regime: Regime::Liang },
    EraEntry { name: "中大同", regime: Regime::Liang },
    EraEntry { name: "太清", regime: Regime::Liang },
    EraEntry { name: "大寶", regime: Regime::Liang },
    EraEntry { name: "承聖", regime: Regime::Liang },
    EraEntry { name: "天成", regime: Regime::Liang },
    // ── 陳 ──
    EraEntry { name: "永定", regime: Regime::Chen },
    EraEntry { name: "天嘉", regime: Regime::Chen },
    EraEntry { name: "天康", regime: Regime::Chen },
    EraEntry { name: "光大", regime: Regime::Chen },
    EraEntry { name: "太建", regime: Regime::Chen },
    EraEntry { name: "至德", regime: Regime::Chen },
    EraEntry { name: "禎明", regime: Regime::Chen },
    // ── 北魏 ──
    EraEntry { name: "登國", regime: Regime::NorthernWei },
    EraEntry { name: "皇始", regime: Regime::NorthernWei },
    EraEntry { name: "天興", regime: Regime::NorthernWei },
    EraEntry { name: "天賜", regime: Regime::NorthernWei },
    EraEntry { name: "永興", regime: Regime::NorthernWei },
    EraEntry { name: "神瑞", regime: Regime::NorthernWei },
    EraEntry { name: "泰常", regime: Regime::NorthernWei },
    EraEntry { name: "始光", regime: Regime::NorthernWei },
    EraEntry { name: "神麚", regime: Regime::NorthernWei },
    EraEntry { name: "延和", regime: Regime::NorthernWei },
    EraEntry { name: "太延", regime: Regime::NorthernWei },
    EraEntry { name: "太平真君", regime: Regime::NorthernWei },
    EraEntry { name: "正平", regime: Regime::NorthernWei },
    EraEntry { name: "興安", regime: Regime::NorthernWei },
    EraEntry { name: "興光", regime: Regime::NorthernWei },
    EraEntry { name: "太安", regime: Regime::NorthernWei },
    EraEntry { name: "和平", regime: Regime::NorthernWei },
    EraEntry { name: "天安", regime: Regime::NorthernWei },
    EraEntry { name: "皇興", regime: Regime::NorthernWei },
    EraEntry { name: "延興", regime: Regime::NorthernWei },
    EraEntry { name: "承明", regime: Regime::NorthernWei },
    EraEntry { name: "太和", regime: Regime::NorthernWei },
    EraEntry { name: "景明", regime: Regime::NorthernWei },
    EraEntry { name: "正始", regime: Regime::NorthernWei },
    EraEntry { name: "永平", regime: Regime::NorthernWei },
    EraEntry { name: "延昌", regime: Regime::NorthernWei },
    EraEntry { name: "熙平", regime: Regime::NorthernWei },
    EraEntry { name: "神龜", regime: Regime::NorthernWei },
    EraEntry { name: "正光", regime: Regime::NorthernWei },
    EraEntry { name: "孝昌", regime: Regime::NorthernWei },
    EraEntry { name: "武泰", regime: Regime::NorthernWei },
    EraEntry { name: "建義", regime: Regime::NorthernWei },
    EraEntry { name: "永安", regime: Regime::NorthernWei },
    EraEntry { name: "建明", regime: Regime::NorthernWei },
    EraEntry { name: "普泰", regime: Regime::NorthernWei },
    EraEntry { name: "中興", regime: Regime::NorthernWei },
    EraEntry { name: "太昌", regime: Regime::NorthernWei },
    EraEntry { name: "永興", regime: Regime::NorthernWei },
    EraEntry { name: "永熙", regime: Regime::NorthernWei },
];

// ── Disambiguation ───────────────────────────────────────────────────

/// Default regime for each Book. Used as first guess for era name
/// disambiguation. The 載記 section of 晉書 references other regimes,
/// which need context-based resolution.
pub fn default_regime(book: Book) -> Regime {
    match book {
        Book::JinShu => Regime::EasternJin, // most of 晉書 is Eastern Jin context
        Book::SongShu => Regime::LiuSong,
        Book::NanQiShu => Regime::SouthernQi,
        Book::LiangShu => Regime::Liang,
        Book::ChenShu => Regime::Chen,
        Book::WeiShu => Regime::NorthernWei,
    }
}

/// Resolve an era name string to a regime, given context (which Book it appears in).
/// Returns all matching (era_name, regime) pairs; caller can use Book context to pick.
pub fn resolve_era(era_name: &str, book: Book) -> Option<Regime> {
    let default = default_regime(book);

    // First try: exact match with the default regime for this book
    for e in ERA_NAMES {
        if e.name == era_name && e.regime == default {
            return Some(e.regime);
        }
    }

    // Second try: any match (cross-regime references, e.g. 晉書 citing 前秦 era)
    for e in ERA_NAMES {
        if e.name == era_name {
            return Some(e.regime);
        }
    }

    None
}

/// Build a regex alternation matching any known era name.
/// Sorted by length descending so "太平真君" matches before "太平".
pub fn build_era_regex() -> String {
    let mut names: Vec<&str> = ERA_NAMES.iter().map(|e| e.name).collect();
    names.sort_by(|a, b| b.chars().count().cmp(&a.chars().count()));
    names.dedup();
    format!("(?:{})", names.join("|"))
}
