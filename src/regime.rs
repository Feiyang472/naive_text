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

/// A single era name belonging to one regime, with exact AD year range.
pub struct EraEntry {
    pub name: &'static str,
    pub regime: Regime,
    /// First AD year of this era (e.g. 424 for 元嘉).
    pub start_ad: u16,
    /// Last AD year of this era (e.g. 453 for 元嘉).
    #[allow(dead_code)]
    pub end_ad: u16,
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
        start_ad: 265,
        end_ad: 274,
    },
    EraEntry {
        name: "咸寧",
        regime: Regime::WesternJin,
        start_ad: 275,
        end_ad: 280,
    },
    EraEntry {
        name: "太康",
        regime: Regime::WesternJin,
        start_ad: 280,
        end_ad: 289,
    },
    EraEntry {
        name: "太熙",
        regime: Regime::WesternJin,
        start_ad: 290,
        end_ad: 290,
    },
    EraEntry {
        name: "永熙",
        regime: Regime::WesternJin,
        start_ad: 290,
        end_ad: 290,
    },
    EraEntry {
        name: "永平",
        regime: Regime::WesternJin,
        start_ad: 291,
        end_ad: 291,
    },
    EraEntry {
        name: "元康",
        regime: Regime::WesternJin,
        start_ad: 291,
        end_ad: 299,
    },
    EraEntry {
        name: "永康",
        regime: Regime::WesternJin,
        start_ad: 300,
        end_ad: 301,
    },
    EraEntry {
        name: "永寧",
        regime: Regime::WesternJin,
        start_ad: 301,
        end_ad: 302,
    },
    EraEntry {
        name: "太安",
        regime: Regime::WesternJin,
        start_ad: 302,
        end_ad: 304,
    },
    EraEntry {
        name: "永安",
        regime: Regime::WesternJin,
        start_ad: 304,
        end_ad: 304,
    },
    EraEntry {
        name: "建武",
        regime: Regime::WesternJin,
        start_ad: 304,
        end_ad: 304,
    },
    EraEntry {
        name: "永興",
        regime: Regime::WesternJin,
        start_ad: 304,
        end_ad: 306,
    },
    EraEntry {
        name: "光熙",
        regime: Regime::WesternJin,
        start_ad: 306,
        end_ad: 306,
    },
    EraEntry {
        name: "永嘉",
        regime: Regime::WesternJin,
        start_ad: 307,
        end_ad: 313,
    },
    EraEntry {
        name: "建興",
        regime: Regime::WesternJin,
        start_ad: 313,
        end_ad: 317,
    },
    // ── 東晉 ──
    EraEntry {
        name: "建武",
        regime: Regime::EasternJin,
        start_ad: 317,
        end_ad: 318,
    },
    EraEntry {
        name: "大興",
        regime: Regime::EasternJin,
        start_ad: 318,
        end_ad: 321,
    },
    EraEntry {
        name: "永昌",
        regime: Regime::EasternJin,
        start_ad: 322,
        end_ad: 323,
    },
    EraEntry {
        name: "太寧",
        regime: Regime::EasternJin,
        start_ad: 323,
        end_ad: 326,
    },
    EraEntry {
        name: "咸和",
        regime: Regime::EasternJin,
        start_ad: 326,
        end_ad: 334,
    },
    EraEntry {
        name: "咸康",
        regime: Regime::EasternJin,
        start_ad: 335,
        end_ad: 342,
    },
    EraEntry {
        name: "建元",
        regime: Regime::EasternJin,
        start_ad: 343,
        end_ad: 344,
    },
    EraEntry {
        name: "永和",
        regime: Regime::EasternJin,
        start_ad: 345,
        end_ad: 356,
    },
    EraEntry {
        name: "升平",
        regime: Regime::EasternJin,
        start_ad: 357,
        end_ad: 361,
    },
    EraEntry {
        name: "隆和",
        regime: Regime::EasternJin,
        start_ad: 362,
        end_ad: 363,
    },
    EraEntry {
        name: "興寧",
        regime: Regime::EasternJin,
        start_ad: 363,
        end_ad: 365,
    },
    EraEntry {
        name: "太和",
        regime: Regime::EasternJin,
        start_ad: 366,
        end_ad: 371,
    },
    EraEntry {
        name: "咸安",
        regime: Regime::EasternJin,
        start_ad: 371,
        end_ad: 372,
    },
    EraEntry {
        name: "寧康",
        regime: Regime::EasternJin,
        start_ad: 373,
        end_ad: 375,
    },
    EraEntry {
        name: "太元",
        regime: Regime::EasternJin,
        start_ad: 376,
        end_ad: 396,
    },
    EraEntry {
        name: "隆安",
        regime: Regime::EasternJin,
        start_ad: 397,
        end_ad: 401,
    },
    EraEntry {
        name: "元興",
        regime: Regime::EasternJin,
        start_ad: 402,
        end_ad: 404,
    },
    EraEntry {
        name: "義熙",
        regime: Regime::EasternJin,
        start_ad: 405,
        end_ad: 418,
    },
    // ── 劉宋 ──
    EraEntry {
        name: "永初",
        regime: Regime::LiuSong,
        start_ad: 420,
        end_ad: 422,
    },
    EraEntry {
        name: "景平",
        regime: Regime::LiuSong,
        start_ad: 423,
        end_ad: 424,
    },
    EraEntry {
        name: "元嘉",
        regime: Regime::LiuSong,
        start_ad: 424,
        end_ad: 453,
    },
    EraEntry {
        name: "孝建",
        regime: Regime::LiuSong,
        start_ad: 454,
        end_ad: 456,
    },
    EraEntry {
        name: "大明",
        regime: Regime::LiuSong,
        start_ad: 457,
        end_ad: 464,
    },
    EraEntry {
        name: "永光",
        regime: Regime::LiuSong,
        start_ad: 465,
        end_ad: 465,
    },
    EraEntry {
        name: "景和",
        regime: Regime::LiuSong,
        start_ad: 465,
        end_ad: 465,
    },
    EraEntry {
        name: "泰始",
        regime: Regime::LiuSong,
        start_ad: 465,
        end_ad: 471,
    },
    EraEntry {
        name: "泰豫",
        regime: Regime::LiuSong,
        start_ad: 472,
        end_ad: 472,
    },
    EraEntry {
        name: "元徽",
        regime: Regime::LiuSong,
        start_ad: 473,
        end_ad: 477,
    },
    EraEntry {
        name: "昇明",
        regime: Regime::LiuSong,
        start_ad: 477,
        end_ad: 479,
    },
    // ── 南齊 ──
    EraEntry {
        name: "建元",
        regime: Regime::SouthernQi,
        start_ad: 479,
        end_ad: 482,
    },
    EraEntry {
        name: "永明",
        regime: Regime::SouthernQi,
        start_ad: 483,
        end_ad: 493,
    },
    EraEntry {
        name: "隆昌",
        regime: Regime::SouthernQi,
        start_ad: 494,
        end_ad: 494,
    },
    EraEntry {
        name: "延興",
        regime: Regime::SouthernQi,
        start_ad: 494,
        end_ad: 494,
    },
    EraEntry {
        name: "建武",
        regime: Regime::SouthernQi,
        start_ad: 494,
        end_ad: 498,
    },
    EraEntry {
        name: "永泰",
        regime: Regime::SouthernQi,
        start_ad: 498,
        end_ad: 498,
    },
    EraEntry {
        name: "永元",
        regime: Regime::SouthernQi,
        start_ad: 499,
        end_ad: 501,
    },
    EraEntry {
        name: "中興",
        regime: Regime::SouthernQi,
        start_ad: 501,
        end_ad: 502,
    },
    // ── 梁 ──
    EraEntry {
        name: "天監",
        regime: Regime::Liang,
        start_ad: 502,
        end_ad: 519,
    },
    EraEntry {
        name: "普通",
        regime: Regime::Liang,
        start_ad: 520,
        end_ad: 527,
    },
    EraEntry {
        name: "大通",
        regime: Regime::Liang,
        start_ad: 527,
        end_ad: 529,
    },
    EraEntry {
        name: "中大通",
        regime: Regime::Liang,
        start_ad: 529,
        end_ad: 534,
    },
    EraEntry {
        name: "大同",
        regime: Regime::Liang,
        start_ad: 535,
        end_ad: 546,
    },
    EraEntry {
        name: "中大同",
        regime: Regime::Liang,
        start_ad: 546,
        end_ad: 547,
    },
    EraEntry {
        name: "太清",
        regime: Regime::Liang,
        start_ad: 547,
        end_ad: 549,
    },
    EraEntry {
        name: "大寶",
        regime: Regime::Liang,
        start_ad: 550,
        end_ad: 551,
    },
    EraEntry {
        name: "承聖",
        regime: Regime::Liang,
        start_ad: 552,
        end_ad: 555,
    },
    EraEntry {
        name: "天成",
        regime: Regime::Liang,
        start_ad: 555,
        end_ad: 555,
    },
    // ── 陳 ──
    EraEntry {
        name: "永定",
        regime: Regime::Chen,
        start_ad: 557,
        end_ad: 559,
    },
    EraEntry {
        name: "天嘉",
        regime: Regime::Chen,
        start_ad: 560,
        end_ad: 566,
    },
    EraEntry {
        name: "天康",
        regime: Regime::Chen,
        start_ad: 566,
        end_ad: 566,
    },
    EraEntry {
        name: "光大",
        regime: Regime::Chen,
        start_ad: 567,
        end_ad: 568,
    },
    EraEntry {
        name: "太建",
        regime: Regime::Chen,
        start_ad: 569,
        end_ad: 582,
    },
    EraEntry {
        name: "至德",
        regime: Regime::Chen,
        start_ad: 583,
        end_ad: 586,
    },
    EraEntry {
        name: "禎明",
        regime: Regime::Chen,
        start_ad: 587,
        end_ad: 589,
    },
    // ── 北魏 ──
    EraEntry {
        name: "登國",
        regime: Regime::NorthernWei,
        start_ad: 386,
        end_ad: 396,
    },
    EraEntry {
        name: "皇始",
        regime: Regime::NorthernWei,
        start_ad: 396,
        end_ad: 398,
    },
    EraEntry {
        name: "天興",
        regime: Regime::NorthernWei,
        start_ad: 398,
        end_ad: 404,
    },
    EraEntry {
        name: "天賜",
        regime: Regime::NorthernWei,
        start_ad: 404,
        end_ad: 409,
    },
    EraEntry {
        name: "永興",
        regime: Regime::NorthernWei,
        start_ad: 409,
        end_ad: 413,
    },
    EraEntry {
        name: "神瑞",
        regime: Regime::NorthernWei,
        start_ad: 414,
        end_ad: 416,
    },
    EraEntry {
        name: "泰常",
        regime: Regime::NorthernWei,
        start_ad: 416,
        end_ad: 423,
    },
    EraEntry {
        name: "始光",
        regime: Regime::NorthernWei,
        start_ad: 424,
        end_ad: 428,
    },
    EraEntry {
        name: "神麚",
        regime: Regime::NorthernWei,
        start_ad: 428,
        end_ad: 431,
    },
    EraEntry {
        name: "延和",
        regime: Regime::NorthernWei,
        start_ad: 432,
        end_ad: 435,
    },
    EraEntry {
        name: "太延",
        regime: Regime::NorthernWei,
        start_ad: 435,
        end_ad: 440,
    },
    EraEntry {
        name: "太平真君",
        regime: Regime::NorthernWei,
        start_ad: 440,
        end_ad: 451,
    },
    EraEntry {
        name: "正平",
        regime: Regime::NorthernWei,
        start_ad: 451,
        end_ad: 452,
    },
    EraEntry {
        name: "興安",
        regime: Regime::NorthernWei,
        start_ad: 452,
        end_ad: 454,
    },
    EraEntry {
        name: "興光",
        regime: Regime::NorthernWei,
        start_ad: 454,
        end_ad: 455,
    },
    EraEntry {
        name: "太安",
        regime: Regime::NorthernWei,
        start_ad: 455,
        end_ad: 459,
    },
    EraEntry {
        name: "和平",
        regime: Regime::NorthernWei,
        start_ad: 460,
        end_ad: 465,
    },
    EraEntry {
        name: "天安",
        regime: Regime::NorthernWei,
        start_ad: 466,
        end_ad: 467,
    },
    EraEntry {
        name: "皇興",
        regime: Regime::NorthernWei,
        start_ad: 467,
        end_ad: 471,
    },
    EraEntry {
        name: "延興",
        regime: Regime::NorthernWei,
        start_ad: 471,
        end_ad: 476,
    },
    EraEntry {
        name: "承明",
        regime: Regime::NorthernWei,
        start_ad: 476,
        end_ad: 476,
    },
    EraEntry {
        name: "太和",
        regime: Regime::NorthernWei,
        start_ad: 477,
        end_ad: 499,
    },
    EraEntry {
        name: "景明",
        regime: Regime::NorthernWei,
        start_ad: 500,
        end_ad: 504,
    },
    EraEntry {
        name: "正始",
        regime: Regime::NorthernWei,
        start_ad: 504,
        end_ad: 508,
    },
    EraEntry {
        name: "永平",
        regime: Regime::NorthernWei,
        start_ad: 508,
        end_ad: 512,
    },
    EraEntry {
        name: "延昌",
        regime: Regime::NorthernWei,
        start_ad: 512,
        end_ad: 515,
    },
    EraEntry {
        name: "熙平",
        regime: Regime::NorthernWei,
        start_ad: 516,
        end_ad: 518,
    },
    EraEntry {
        name: "神龜",
        regime: Regime::NorthernWei,
        start_ad: 518,
        end_ad: 520,
    },
    EraEntry {
        name: "正光",
        regime: Regime::NorthernWei,
        start_ad: 520,
        end_ad: 525,
    },
    EraEntry {
        name: "孝昌",
        regime: Regime::NorthernWei,
        start_ad: 525,
        end_ad: 528,
    },
    EraEntry {
        name: "武泰",
        regime: Regime::NorthernWei,
        start_ad: 528,
        end_ad: 528,
    },
    EraEntry {
        name: "建義",
        regime: Regime::NorthernWei,
        start_ad: 528,
        end_ad: 528,
    },
    EraEntry {
        name: "永安",
        regime: Regime::NorthernWei,
        start_ad: 528,
        end_ad: 530,
    },
    EraEntry {
        name: "建明",
        regime: Regime::NorthernWei,
        start_ad: 530,
        end_ad: 531,
    },
    EraEntry {
        name: "普泰",
        regime: Regime::NorthernWei,
        start_ad: 531,
        end_ad: 531,
    },
    EraEntry {
        name: "中興",
        regime: Regime::NorthernWei,
        start_ad: 531,
        end_ad: 532,
    },
    EraEntry {
        name: "太昌",
        regime: Regime::NorthernWei,
        start_ad: 532,
        end_ad: 532,
    },
    EraEntry {
        name: "永興",
        regime: Regime::NorthernWei,
        start_ad: 409,
        end_ad: 413,
    },
    EraEntry {
        name: "永熙",
        regime: Regime::NorthernWei,
        start_ad: 532,
        end_ad: 534,
    },
    // ── 漢趙 (前趙) ──
    EraEntry {
        name: "元熙",
        regime: Regime::HanZhao,
        start_ad: 304,
        end_ad: 308,
    },
    EraEntry {
        name: "河瑞",
        regime: Regime::HanZhao,
        start_ad: 309,
        end_ad: 310,
    },
    EraEntry {
        name: "光興",
        regime: Regime::HanZhao,
        start_ad: 310,
        end_ad: 311,
    },
    EraEntry {
        name: "嘉平",
        regime: Regime::HanZhao,
        start_ad: 311,
        end_ad: 315,
    },
    EraEntry {
        name: "麟嘉",
        regime: Regime::HanZhao,
        start_ad: 316,
        end_ad: 318,
    },
    EraEntry {
        name: "光初",
        regime: Regime::HanZhao,
        start_ad: 318,
        end_ad: 329,
    },
    // ── 後趙 ──
    EraEntry {
        name: "建平",
        regime: Regime::LaterZhao,
        start_ad: 330,
        end_ad: 333,
    },
    EraEntry {
        name: "建武",
        regime: Regime::LaterZhao,
        start_ad: 335,
        end_ad: 348,
    },
    EraEntry {
        name: "太寧",
        regime: Regime::LaterZhao,
        start_ad: 349,
        end_ad: 349,
    },
    EraEntry {
        name: "青龍",
        regime: Regime::LaterZhao,
        start_ad: 350,
        end_ad: 350,
    },
    // ── 成漢 ──
    EraEntry {
        name: "太武",
        regime: Regime::ChengHan,
        start_ad: 303,
        end_ad: 304,
    },
    EraEntry {
        name: "晏平",
        regime: Regime::ChengHan,
        start_ad: 306,
        end_ad: 310,
    },
    EraEntry {
        name: "玉衡",
        regime: Regime::ChengHan,
        start_ad: 311,
        end_ad: 334,
    },
    EraEntry {
        name: "漢興",
        regime: Regime::ChengHan,
        start_ad: 338,
        end_ad: 343,
    },
    // ── 前涼 ──
    EraEntry {
        name: "太初",
        regime: Regime::FormerLiang,
        start_ad: 314,
        end_ad: 320,
    },
    EraEntry {
        name: "建興",
        regime: Regime::FormerLiang,
        start_ad: 317,
        end_ad: 320,
    },
    // ── 前燕 ──
    EraEntry {
        name: "元璽",
        regime: Regime::FormerYan,
        start_ad: 352,
        end_ad: 357,
    },
    EraEntry {
        name: "光壽",
        regime: Regime::FormerYan,
        start_ad: 357,
        end_ad: 360,
    },
    EraEntry {
        name: "建熙",
        regime: Regime::FormerYan,
        start_ad: 360,
        end_ad: 370,
    },
    // ── 前秦 ──
    EraEntry {
        name: "皇始",
        regime: Regime::FormerQin,
        start_ad: 351,
        end_ad: 355,
    },
    EraEntry {
        name: "壽光",
        regime: Regime::FormerQin,
        start_ad: 355,
        end_ad: 357,
    },
    EraEntry {
        name: "甘露",
        regime: Regime::FormerQin,
        start_ad: 359,
        end_ad: 364,
    },
    EraEntry {
        name: "建元",
        regime: Regime::FormerQin,
        start_ad: 365,
        end_ad: 385,
    },
    EraEntry {
        name: "太初",
        regime: Regime::FormerQin,
        start_ad: 386,
        end_ad: 394,
    },
    // ── 後秦 ──
    EraEntry {
        name: "建初",
        regime: Regime::LaterQin,
        start_ad: 386,
        end_ad: 394,
    },
    EraEntry {
        name: "皇初",
        regime: Regime::LaterQin,
        start_ad: 394,
        end_ad: 399,
    },
    EraEntry {
        name: "弘始",
        regime: Regime::LaterQin,
        start_ad: 399,
        end_ad: 416,
    },
    // ── 後燕 ──
    EraEntry {
        name: "建興",
        regime: Regime::LaterYan,
        start_ad: 386,
        end_ad: 396,
    },
    EraEntry {
        name: "長樂",
        regime: Regime::LaterYan,
        start_ad: 399,
        end_ad: 401,
    },
    EraEntry {
        name: "光始",
        regime: Regime::LaterYan,
        start_ad: 401,
        end_ad: 406,
    },
    EraEntry {
        name: "建始",
        regime: Regime::LaterYan,
        start_ad: 407,
        end_ad: 407,
    },
    // ── 西秦 ──
    EraEntry {
        name: "建義",
        regime: Regime::WesternQin,
        start_ad: 385,
        end_ad: 388,
    },
    EraEntry {
        name: "太初",
        regime: Regime::WesternQin,
        start_ad: 388,
        end_ad: 400,
    },
    EraEntry {
        name: "更始",
        regime: Regime::WesternQin,
        start_ad: 409,
        end_ad: 412,
    },
    EraEntry {
        name: "建弘",
        regime: Regime::WesternQin,
        start_ad: 420,
        end_ad: 428,
    },
    // ── 後涼 ──
    EraEntry {
        name: "太安",
        regime: Regime::LaterLiang,
        start_ad: 386,
        end_ad: 389,
    },
    EraEntry {
        name: "麟嘉",
        regime: Regime::LaterLiang,
        start_ad: 389,
        end_ad: 396,
    },
    EraEntry {
        name: "龍飛",
        regime: Regime::LaterLiang,
        start_ad: 396,
        end_ad: 399,
    },
    // ── 南涼 ──
    EraEntry {
        name: "太初",
        regime: Regime::SouthernLiang,
        start_ad: 397,
        end_ad: 399,
    },
    EraEntry {
        name: "建和",
        regime: Regime::SouthernLiang,
        start_ad: 400,
        end_ad: 402,
    },
    EraEntry {
        name: "弘昌",
        regime: Regime::SouthernLiang,
        start_ad: 402,
        end_ad: 404,
    },
    EraEntry {
        name: "嘉平",
        regime: Regime::SouthernLiang,
        start_ad: 408,
        end_ad: 414,
    },
    // ── 南燕 ──
    EraEntry {
        name: "建平",
        regime: Regime::SouthernYan,
        start_ad: 400,
        end_ad: 405,
    },
    EraEntry {
        name: "太上",
        regime: Regime::SouthernYan,
        start_ad: 405,
        end_ad: 410,
    },
    // ── 西涼 ──
    EraEntry {
        name: "庚子",
        regime: Regime::WesternLiang,
        start_ad: 400,
        end_ad: 404,
    },
    EraEntry {
        name: "建初",
        regime: Regime::WesternLiang,
        start_ad: 405,
        end_ad: 417,
    },
    // ── 北涼 ──
    EraEntry {
        name: "神璽",
        regime: Regime::NorthernLiang,
        start_ad: 397,
        end_ad: 399,
    },
    EraEntry {
        name: "天璽",
        regime: Regime::NorthernLiang,
        start_ad: 399,
        end_ad: 401,
    },
    EraEntry {
        name: "永安",
        regime: Regime::NorthernLiang,
        start_ad: 401,
        end_ad: 412,
    },
    EraEntry {
        name: "玄始",
        regime: Regime::NorthernLiang,
        start_ad: 412,
        end_ad: 427,
    },
    EraEntry {
        name: "承平",
        regime: Regime::NorthernLiang,
        start_ad: 443,
        end_ad: 460,
    },
    // ── 夏 (赫連夏) ──
    EraEntry {
        name: "龍昇",
        regime: Regime::XiaState,
        start_ad: 407,
        end_ad: 413,
    },
    EraEntry {
        name: "鳳翔",
        regime: Regime::XiaState,
        start_ad: 413,
        end_ad: 418,
    },
    EraEntry {
        name: "昌武",
        regime: Regime::XiaState,
        start_ad: 418,
        end_ad: 419,
    },
    EraEntry {
        name: "真興",
        regime: Regime::XiaState,
        start_ad: 419,
        end_ad: 425,
    },
    EraEntry {
        name: "承光",
        regime: Regime::XiaState,
        start_ad: 425,
        end_ad: 428,
    },
    // ── 北燕 ──
    EraEntry {
        name: "正始",
        regime: Regime::NorthernYan,
        start_ad: 407,
        end_ad: 409,
    },
    EraEntry {
        name: "太平",
        regime: Regime::NorthernYan,
        start_ad: 409,
        end_ad: 430,
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
