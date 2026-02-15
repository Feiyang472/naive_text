use serde::Serialize;
use std::path::PathBuf;

// ── Which historical book ──────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum Book {
    /// 晉書
    JinShu,
    /// 宋書
    SongShu,
    /// 南齊書
    NanQiShu,
    /// 梁書
    LiangShu,
    /// 陳書
    ChenShu,
    /// 魏書
    WeiShu,
}

impl Book {
    pub fn from_dir_name(name: &str) -> Option<Self> {
        match name {
            "晉書" => Some(Self::JinShu),
            "宋書" => Some(Self::SongShu),
            "南齊書" => Some(Self::NanQiShu),
            "梁書" => Some(Self::LiangShu),
            "陳書" => Some(Self::ChenShu),
            "魏書" => Some(Self::WeiShu),
            _ => None,
        }
    }

    pub fn as_chinese(&self) -> &'static str {
        match self {
            Self::JinShu => "晉書",
            Self::SongShu => "宋書",
            Self::NanQiShu => "南齊書",
            Self::LiangShu => "梁書",
            Self::ChenShu => "陳書",
            Self::WeiShu => "魏書",
        }
    }
}

// ── What section of the book ───────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum Section {
    /// 本紀/帝紀 – imperial annals
    BenJi,
    /// 列傳 – biographies
    LieZhuan,
    /// 載記 – records of foreign/rival states (晉書 only)
    ZaiJi,
    /// 志 – treatises
    Zhi,
    /// Other (附錄, etc.)
    Other,
}

impl Section {
    pub fn from_dir_name(name: &str) -> Self {
        if name.contains("本紀") || name.contains("紀") {
            Self::BenJi
        } else if name.contains("列傳") {
            Self::LieZhuan
        } else if name.contains("載記") {
            Self::ZaiJi
        } else if name.contains("志") {
            Self::Zhi
        } else {
            Self::Other
        }
    }
}

// ── Source location in the corpus ──────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct Source {
    pub book: Book,
    pub section: Section,
    /// Human-readable juan name, e.g. "列傳第四　褚淵"
    pub juan: String,
    pub file_path: PathBuf,
}

// ── Courtesy name: distinguish "recorded" from "not recorded" ─────

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", content = "value")]
pub enum CourtesyName {
    Recorded(String),
    NotRecorded,
}

// ── Childhood / informal name ─────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", content = "value")]
pub enum ChildhoodName {
    Recorded(String),
    NotRecorded,
}

// ── The type of person determines what fields are available ────────

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum PersonKind {
    /// Emperor – has temple name, posthumous title
    Emperor {
        /// 庙号: 高祖, 太宗, 世祖, …
        temple_name: Option<String>,
        /// 谥号: 武皇帝, 宣帝, …
        posthumous_title: String,
        /// 諱
        given_name: String,
        /// 姓 (may be implicit from the book's ruling house)
        surname: Option<String>,
        courtesy_name: CourtesyName,
        childhood_name: ChildhoodName,
    },
    /// Regular official/person in biographies
    Official {
        surname: String,
        given_name: String,
        courtesy_name: CourtesyName,
        origin: Option<String>,
    },
    /// Deposed emperor / prince with 諱 but no full temple name
    Deposed {
        title: String,
        given_name: String,
        courtesy_name: CourtesyName,
        childhood_name: ChildhoodName,
    },
    /// Ruler of a rival/foreign state (載記 figures: 十六國 etc.)
    /// Not recognized as "emperor" by the compiling dynasty, but
    /// was sovereign of an independent polity.
    Ruler {
        surname: String,
        given_name: String,
        courtesy_name: CourtesyName,
        /// Lineage/origin description, e.g. "皝之第五子", "新興匈奴"
        lineage: Option<String>,
    },
}

// ── A fully identified historical person ──────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct Person {
    pub kind: PersonKind,
    pub source: Source,
    /// All known ways this person is referred to in text
    pub aliases: Vec<String>,
}

impl Person {
    /// The canonical display name for this person
    pub fn display_name(&self) -> String {
        match &self.kind {
            PersonKind::Emperor {
                surname,
                given_name,
                posthumous_title,
                ..
            } => {
                if let Some(s) = surname {
                    format!("{s}{given_name}")
                } else {
                    posthumous_title.clone()
                }
            }
            PersonKind::Official {
                surname,
                given_name,
                ..
            } => format!("{surname}{given_name}"),
            PersonKind::Deposed {
                title, given_name, ..
            } => format!("{title}{given_name}"),
            PersonKind::Ruler {
                surname,
                given_name,
                ..
            } => format!("{surname}{given_name}"),
        }
    }

    /// Collect all the names/aliases this person might be referred to
    pub fn compute_aliases(&mut self) {
        let mut aliases = Vec::new();

        match &self.kind {
            PersonKind::Emperor {
                temple_name,
                posthumous_title,
                given_name,
                surname,
                courtesy_name,
                childhood_name,
            } => {
                // Full name if surname known
                if let Some(s) = surname {
                    aliases.push(format!("{s}{given_name}"));
                }
                aliases.push(given_name.clone());
                aliases.push(posthumous_title.clone());
                if let Some(t) = temple_name {
                    aliases.push(t.clone());
                }
                if let CourtesyName::Recorded(c) = courtesy_name {
                    aliases.push(c.clone());
                }
                if let ChildhoodName::Recorded(c) = childhood_name {
                    aliases.push(c.clone());
                }
            }
            PersonKind::Official {
                surname,
                given_name,
                courtesy_name,
                ..
            } => {
                aliases.push(format!("{surname}{given_name}"));
                aliases.push(given_name.clone());
                if let CourtesyName::Recorded(c) = courtesy_name {
                    aliases.push(c.clone());
                }
            }
            PersonKind::Deposed {
                title,
                given_name,
                courtesy_name,
                childhood_name,
            } => {
                aliases.push(title.clone());
                aliases.push(given_name.clone());
                if let CourtesyName::Recorded(c) = courtesy_name {
                    aliases.push(c.clone());
                }
                if let ChildhoodName::Recorded(c) = childhood_name {
                    aliases.push(c.clone());
                }
            }
            PersonKind::Ruler {
                surname,
                given_name,
                courtesy_name,
                ..
            } => {
                aliases.push(format!("{surname}{given_name}"));
                aliases.push(given_name.clone());
                if let CourtesyName::Recorded(c) = courtesy_name {
                    aliases.push(c.clone());
                }
            }
        }

        self.aliases = aliases;
    }
}

// ── How a person is referenced in running text ────────────────────

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "ref_type")]
pub enum PersonRef {
    /// 褚淵
    FullName { surname: String, given: String },
    /// 淵 (in the context of 褚淵's biography)
    GivenOnly(String),
    /// 彥回
    CourtesyOnly(String),
    /// 尚書令, 司空, etc.
    Title(String),
    /// 高祖宣帝 – only for emperors
    TemplePosthumous { temple: String, posthumous: String },
    /// Omitted subject – defaults to biography subject
    SubjectOmitted,
}
