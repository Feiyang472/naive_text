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
    HanZhao,       // 漢趙 (前趙)
    LaterZhao,     // 後趙
    ChengHan,      // 成漢
    FormerLiang,   // 前涼
    FormerYan,     // 前燕
    FormerQin,     // 前秦
    LaterQin,      // 後秦
    LaterYan,      // 後燕
    WesternQin,    // 西秦
    LaterLiang,    // 後涼
    SouthernLiang, // 南涼
    SouthernYan,   // 南燕
    WesternLiang,  // 西涼
    NorthernLiang, // 北涼
    XiaState,      // 夏 (赫連夏)
    NorthernYan,   // 北燕
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

    /// Approximate AD start year, for sorting concurrent regimes.
    pub fn start_ad_year(&self) -> u16 {
        match self {
            Self::WesternJin => 265,
            Self::EasternJin => 317,
            Self::LiuSong => 420,
            Self::SouthernQi => 479,
            Self::Liang => 502,
            Self::Chen => 557,
            Self::NorthernWei => 386,
            Self::HanZhao => 304,
            Self::LaterZhao => 319,
            Self::ChengHan => 304,
            Self::FormerLiang => 314,
            Self::FormerYan => 337,
            Self::FormerQin => 351,
            Self::LaterQin => 384,
            Self::LaterYan => 384,
            Self::WesternQin => 385,
            Self::LaterLiang => 386,
            Self::SouthernLiang => 397,
            Self::SouthernYan => 398,
            Self::WesternLiang => 400,
            Self::NorthernLiang => 397,
            Self::XiaState => 407,
            Self::NorthernYan => 407,
        }
    }
}

// ── Display tree for timeline command ────────────────────────────────

/// A node in the regime display DAG for the `timeline` command.
pub enum DisplayTree {
    /// A single regime with no sub-branches.
    Leaf(Regime),
    /// A regime followed by sequential successors and concurrent branches.
    Branch {
        regime: Regime,
        /// Sequential successors on the same political line.
        sequence: Vec<DisplayTree>,
        /// Concurrent regimes (shown as branching off).
        concurrent: Vec<DisplayTree>,
    },
}

/// Build the canonical display tree for Six Dynasties regimes.
///
/// Structure:
///   Western Jin
///   ├─ Eastern Jin (+ Sixteen Kingdoms as concurrent branches)
///   │  → Liu Song → Southern Qi → Liang → Chen
///   └─ Northern Wei
pub fn regime_display_tree() -> DisplayTree {
    use Regime::*;

    // Sixteen Kingdoms — concurrent with Eastern Jin, sorted by start year
    let mut sixteen_kingdoms: Vec<DisplayTree> = vec![
        DisplayTree::Leaf(HanZhao),
        DisplayTree::Leaf(ChengHan),
        DisplayTree::Leaf(FormerLiang),
        DisplayTree::Leaf(LaterZhao),
        DisplayTree::Leaf(FormerYan),
        DisplayTree::Leaf(FormerQin),
        DisplayTree::Leaf(LaterQin),
        DisplayTree::Leaf(LaterYan),
        DisplayTree::Leaf(WesternQin),
        DisplayTree::Leaf(LaterLiang),
        DisplayTree::Leaf(SouthernLiang),
        DisplayTree::Leaf(NorthernLiang),
        DisplayTree::Leaf(SouthernYan),
        DisplayTree::Leaf(WesternLiang),
        DisplayTree::Leaf(XiaState),
        DisplayTree::Leaf(NorthernYan),
    ];
    sixteen_kingdoms.sort_by_key(|n| match n {
        DisplayTree::Leaf(r) | DisplayTree::Branch { regime: r, .. } => r.start_ad_year(),
    });

    DisplayTree::Branch {
        regime: WesternJin,
        sequence: vec![],
        concurrent: vec![
            DisplayTree::Branch {
                regime: EasternJin,
                sequence: vec![
                    DisplayTree::Leaf(LiuSong),
                    DisplayTree::Leaf(SouthernQi),
                    DisplayTree::Leaf(Liang),
                    DisplayTree::Leaf(Chen),
                ],
                concurrent: sixteen_kingdoms,
            },
            DisplayTree::Leaf(NorthernWei),
        ],
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
    EraEntry {
        name: "泰始",
        regime: Regime::WesternJin,
    },
    EraEntry {
        name: "咸寧",
        regime: Regime::WesternJin,
    },
    EraEntry {
        name: "太康",
        regime: Regime::WesternJin,
    },
    EraEntry {
        name: "太熙",
        regime: Regime::WesternJin,
    },
    EraEntry {
        name: "永熙",
        regime: Regime::WesternJin,
    },
    EraEntry {
        name: "永平",
        regime: Regime::WesternJin,
    },
    EraEntry {
        name: "元康",
        regime: Regime::WesternJin,
    },
    EraEntry {
        name: "永康",
        regime: Regime::WesternJin,
    },
    EraEntry {
        name: "永寧",
        regime: Regime::WesternJin,
    },
    EraEntry {
        name: "太安",
        regime: Regime::WesternJin,
    },
    EraEntry {
        name: "永安",
        regime: Regime::WesternJin,
    },
    EraEntry {
        name: "建武",
        regime: Regime::WesternJin,
    },
    EraEntry {
        name: "永興",
        regime: Regime::WesternJin,
    },
    EraEntry {
        name: "光熙",
        regime: Regime::WesternJin,
    },
    EraEntry {
        name: "永嘉",
        regime: Regime::WesternJin,
    },
    EraEntry {
        name: "建興",
        regime: Regime::WesternJin,
    },
    // ── 東晉 ──
    EraEntry {
        name: "建武",
        regime: Regime::EasternJin,
    },
    EraEntry {
        name: "大興",
        regime: Regime::EasternJin,
    },
    EraEntry {
        name: "永昌",
        regime: Regime::EasternJin,
    },
    EraEntry {
        name: "太寧",
        regime: Regime::EasternJin,
    },
    EraEntry {
        name: "咸和",
        regime: Regime::EasternJin,
    },
    EraEntry {
        name: "咸康",
        regime: Regime::EasternJin,
    },
    EraEntry {
        name: "建元",
        regime: Regime::EasternJin,
    },
    EraEntry {
        name: "永和",
        regime: Regime::EasternJin,
    },
    EraEntry {
        name: "升平",
        regime: Regime::EasternJin,
    },
    EraEntry {
        name: "隆和",
        regime: Regime::EasternJin,
    },
    EraEntry {
        name: "興寧",
        regime: Regime::EasternJin,
    },
    EraEntry {
        name: "太和",
        regime: Regime::EasternJin,
    },
    EraEntry {
        name: "咸安",
        regime: Regime::EasternJin,
    },
    EraEntry {
        name: "寧康",
        regime: Regime::EasternJin,
    },
    EraEntry {
        name: "太元",
        regime: Regime::EasternJin,
    },
    EraEntry {
        name: "隆安",
        regime: Regime::EasternJin,
    },
    EraEntry {
        name: "元興",
        regime: Regime::EasternJin,
    },
    EraEntry {
        name: "義熙",
        regime: Regime::EasternJin,
    },
    // ── 劉宋 ──
    EraEntry {
        name: "永初",
        regime: Regime::LiuSong,
    },
    EraEntry {
        name: "景平",
        regime: Regime::LiuSong,
    },
    EraEntry {
        name: "元嘉",
        regime: Regime::LiuSong,
    },
    EraEntry {
        name: "孝建",
        regime: Regime::LiuSong,
    },
    EraEntry {
        name: "大明",
        regime: Regime::LiuSong,
    },
    EraEntry {
        name: "永光",
        regime: Regime::LiuSong,
    },
    EraEntry {
        name: "景和",
        regime: Regime::LiuSong,
    },
    EraEntry {
        name: "泰始",
        regime: Regime::LiuSong,
    },
    EraEntry {
        name: "泰豫",
        regime: Regime::LiuSong,
    },
    EraEntry {
        name: "元徽",
        regime: Regime::LiuSong,
    },
    EraEntry {
        name: "昇明",
        regime: Regime::LiuSong,
    },
    // ── 南齊 ──
    EraEntry {
        name: "建元",
        regime: Regime::SouthernQi,
    },
    EraEntry {
        name: "永明",
        regime: Regime::SouthernQi,
    },
    EraEntry {
        name: "隆昌",
        regime: Regime::SouthernQi,
    },
    EraEntry {
        name: "延興",
        regime: Regime::SouthernQi,
    },
    EraEntry {
        name: "建武",
        regime: Regime::SouthernQi,
    },
    EraEntry {
        name: "永泰",
        regime: Regime::SouthernQi,
    },
    EraEntry {
        name: "永元",
        regime: Regime::SouthernQi,
    },
    EraEntry {
        name: "中興",
        regime: Regime::SouthernQi,
    },
    // ── 梁 ──
    EraEntry {
        name: "天監",
        regime: Regime::Liang,
    },
    EraEntry {
        name: "普通",
        regime: Regime::Liang,
    },
    EraEntry {
        name: "大通",
        regime: Regime::Liang,
    },
    EraEntry {
        name: "中大通",
        regime: Regime::Liang,
    },
    EraEntry {
        name: "大同",
        regime: Regime::Liang,
    },
    EraEntry {
        name: "中大同",
        regime: Regime::Liang,
    },
    EraEntry {
        name: "太清",
        regime: Regime::Liang,
    },
    EraEntry {
        name: "大寶",
        regime: Regime::Liang,
    },
    EraEntry {
        name: "承聖",
        regime: Regime::Liang,
    },
    EraEntry {
        name: "天成",
        regime: Regime::Liang,
    },
    // ── 陳 ──
    EraEntry {
        name: "永定",
        regime: Regime::Chen,
    },
    EraEntry {
        name: "天嘉",
        regime: Regime::Chen,
    },
    EraEntry {
        name: "天康",
        regime: Regime::Chen,
    },
    EraEntry {
        name: "光大",
        regime: Regime::Chen,
    },
    EraEntry {
        name: "太建",
        regime: Regime::Chen,
    },
    EraEntry {
        name: "至德",
        regime: Regime::Chen,
    },
    EraEntry {
        name: "禎明",
        regime: Regime::Chen,
    },
    // ── 北魏 ──
    EraEntry {
        name: "登國",
        regime: Regime::NorthernWei,
    },
    EraEntry {
        name: "皇始",
        regime: Regime::NorthernWei,
    },
    EraEntry {
        name: "天興",
        regime: Regime::NorthernWei,
    },
    EraEntry {
        name: "天賜",
        regime: Regime::NorthernWei,
    },
    EraEntry {
        name: "永興",
        regime: Regime::NorthernWei,
    },
    EraEntry {
        name: "神瑞",
        regime: Regime::NorthernWei,
    },
    EraEntry {
        name: "泰常",
        regime: Regime::NorthernWei,
    },
    EraEntry {
        name: "始光",
        regime: Regime::NorthernWei,
    },
    EraEntry {
        name: "神麚",
        regime: Regime::NorthernWei,
    },
    EraEntry {
        name: "延和",
        regime: Regime::NorthernWei,
    },
    EraEntry {
        name: "太延",
        regime: Regime::NorthernWei,
    },
    EraEntry {
        name: "太平真君",
        regime: Regime::NorthernWei,
    },
    EraEntry {
        name: "正平",
        regime: Regime::NorthernWei,
    },
    EraEntry {
        name: "興安",
        regime: Regime::NorthernWei,
    },
    EraEntry {
        name: "興光",
        regime: Regime::NorthernWei,
    },
    EraEntry {
        name: "太安",
        regime: Regime::NorthernWei,
    },
    EraEntry {
        name: "和平",
        regime: Regime::NorthernWei,
    },
    EraEntry {
        name: "天安",
        regime: Regime::NorthernWei,
    },
    EraEntry {
        name: "皇興",
        regime: Regime::NorthernWei,
    },
    EraEntry {
        name: "延興",
        regime: Regime::NorthernWei,
    },
    EraEntry {
        name: "承明",
        regime: Regime::NorthernWei,
    },
    EraEntry {
        name: "太和",
        regime: Regime::NorthernWei,
    },
    EraEntry {
        name: "景明",
        regime: Regime::NorthernWei,
    },
    EraEntry {
        name: "正始",
        regime: Regime::NorthernWei,
    },
    EraEntry {
        name: "永平",
        regime: Regime::NorthernWei,
    },
    EraEntry {
        name: "延昌",
        regime: Regime::NorthernWei,
    },
    EraEntry {
        name: "熙平",
        regime: Regime::NorthernWei,
    },
    EraEntry {
        name: "神龜",
        regime: Regime::NorthernWei,
    },
    EraEntry {
        name: "正光",
        regime: Regime::NorthernWei,
    },
    EraEntry {
        name: "孝昌",
        regime: Regime::NorthernWei,
    },
    EraEntry {
        name: "武泰",
        regime: Regime::NorthernWei,
    },
    EraEntry {
        name: "建義",
        regime: Regime::NorthernWei,
    },
    EraEntry {
        name: "永安",
        regime: Regime::NorthernWei,
    },
    EraEntry {
        name: "建明",
        regime: Regime::NorthernWei,
    },
    EraEntry {
        name: "普泰",
        regime: Regime::NorthernWei,
    },
    EraEntry {
        name: "中興",
        regime: Regime::NorthernWei,
    },
    EraEntry {
        name: "太昌",
        regime: Regime::NorthernWei,
    },
    EraEntry {
        name: "永興",
        regime: Regime::NorthernWei,
    },
    EraEntry {
        name: "永熙",
        regime: Regime::NorthernWei,
    },
    // ── 漢趙 (前趙) ──
    EraEntry {
        name: "元熙",
        regime: Regime::HanZhao,
    },
    EraEntry {
        name: "河瑞",
        regime: Regime::HanZhao,
    },
    EraEntry {
        name: "光興",
        regime: Regime::HanZhao,
    },
    EraEntry {
        name: "嘉平",
        regime: Regime::HanZhao,
    },
    EraEntry {
        name: "麟嘉",
        regime: Regime::HanZhao,
    },
    EraEntry {
        name: "光初",
        regime: Regime::HanZhao,
    },
    // ── 後趙 ──
    EraEntry {
        name: "建平",
        regime: Regime::LaterZhao,
    },
    EraEntry {
        name: "建武",
        regime: Regime::LaterZhao,
    },
    EraEntry {
        name: "太寧",
        regime: Regime::LaterZhao,
    },
    EraEntry {
        name: "青龍",
        regime: Regime::LaterZhao,
    },
    // ── 成漢 ──
    EraEntry {
        name: "太武",
        regime: Regime::ChengHan,
    },
    EraEntry {
        name: "晏平",
        regime: Regime::ChengHan,
    },
    EraEntry {
        name: "玉衡",
        regime: Regime::ChengHan,
    },
    EraEntry {
        name: "漢興",
        regime: Regime::ChengHan,
    },
    // ── 前涼 ──
    EraEntry {
        name: "太初",
        regime: Regime::FormerLiang,
    },
    EraEntry {
        name: "建興",
        regime: Regime::FormerLiang,
    },
    // ── 前燕 ──
    EraEntry {
        name: "元璽",
        regime: Regime::FormerYan,
    },
    EraEntry {
        name: "光壽",
        regime: Regime::FormerYan,
    },
    EraEntry {
        name: "建熙",
        regime: Regime::FormerYan,
    },
    // ── 前秦 ──
    EraEntry {
        name: "皇始",
        regime: Regime::FormerQin,
    },
    EraEntry {
        name: "壽光",
        regime: Regime::FormerQin,
    },
    EraEntry {
        name: "甘露",
        regime: Regime::FormerQin,
    },
    EraEntry {
        name: "建元",
        regime: Regime::FormerQin,
    },
    EraEntry {
        name: "太初",
        regime: Regime::FormerQin,
    },
    // ── 後秦 ──
    EraEntry {
        name: "建初",
        regime: Regime::LaterQin,
    },
    EraEntry {
        name: "皇初",
        regime: Regime::LaterQin,
    },
    EraEntry {
        name: "弘始",
        regime: Regime::LaterQin,
    },
    // ── 後燕 ──
    EraEntry {
        name: "建興",
        regime: Regime::LaterYan,
    },
    EraEntry {
        name: "長樂",
        regime: Regime::LaterYan,
    },
    EraEntry {
        name: "光始",
        regime: Regime::LaterYan,
    },
    EraEntry {
        name: "建始",
        regime: Regime::LaterYan,
    },
    // ── 西秦 ──
    EraEntry {
        name: "建義",
        regime: Regime::WesternQin,
    },
    EraEntry {
        name: "太初",
        regime: Regime::WesternQin,
    },
    EraEntry {
        name: "更始",
        regime: Regime::WesternQin,
    },
    EraEntry {
        name: "建弘",
        regime: Regime::WesternQin,
    },
    // ── 後涼 ──
    EraEntry {
        name: "太安",
        regime: Regime::LaterLiang,
    },
    EraEntry {
        name: "麟嘉",
        regime: Regime::LaterLiang,
    },
    EraEntry {
        name: "龍飛",
        regime: Regime::LaterLiang,
    },
    // ── 南涼 ──
    EraEntry {
        name: "太初",
        regime: Regime::SouthernLiang,
    },
    EraEntry {
        name: "建和",
        regime: Regime::SouthernLiang,
    },
    EraEntry {
        name: "弘昌",
        regime: Regime::SouthernLiang,
    },
    EraEntry {
        name: "嘉平",
        regime: Regime::SouthernLiang,
    },
    // ── 南燕 ──
    EraEntry {
        name: "建平",
        regime: Regime::SouthernYan,
    },
    EraEntry {
        name: "太上",
        regime: Regime::SouthernYan,
    },
    // ── 西涼 ──
    EraEntry {
        name: "庚子",
        regime: Regime::WesternLiang,
    },
    EraEntry {
        name: "建初",
        regime: Regime::WesternLiang,
    },
    // ── 北涼 ──
    EraEntry {
        name: "神璽",
        regime: Regime::NorthernLiang,
    },
    EraEntry {
        name: "天璽",
        regime: Regime::NorthernLiang,
    },
    EraEntry {
        name: "永安",
        regime: Regime::NorthernLiang,
    },
    EraEntry {
        name: "玄始",
        regime: Regime::NorthernLiang,
    },
    EraEntry {
        name: "承平",
        regime: Regime::NorthernLiang,
    },
    // ── 夏 (赫連夏) ──
    EraEntry {
        name: "龍昇",
        regime: Regime::XiaState,
    },
    EraEntry {
        name: "鳳翔",
        regime: Regime::XiaState,
    },
    EraEntry {
        name: "昌武",
        regime: Regime::XiaState,
    },
    EraEntry {
        name: "真興",
        regime: Regime::XiaState,
    },
    EraEntry {
        name: "承光",
        regime: Regime::XiaState,
    },
    // ── 北燕 ──
    EraEntry {
        name: "正始",
        regime: Regime::NorthernYan,
    },
    EraEntry {
        name: "太平",
        regime: Regime::NorthernYan,
    },
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
    names.sort_by_key(|b| std::cmp::Reverse(b.chars().count()));
    names.dedup();
    format!("(?:{})", names.join("|"))
}
