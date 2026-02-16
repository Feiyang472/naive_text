/// Title suffixes: the final 2-3 characters of compound official titles.
/// E.g. "前將軍" ends with "將軍", "青州刺史" ends with "刺史".
/// These are used as anchors: when a title suffix appears, the next 2-4 chars
/// are likely a person name.
pub const TITLE_SUFFIXES: &[&str] = &[
    // Military
    "將軍", "校尉", "都尉", "護軍",
    "司馬", // Note: also a compound surname; context disambiguates
    "參軍", // Provincial / local
    "刺史", "太守", "內史", "長史", "別駕", "從事", "主簿", "功曹",
    // Central government
    "尚書", "侍郎", "中郎", "僕射", "常侍", "給事", "令史", "祭酒", "博士",
    // Censorate
    "中丞",
];

/// Standalone titles: complete titles that are NOT suffixes of longer titles.
/// These appear as-is immediately before a person name.
pub const STANDALONE_TITLES: &[&str] = &[
    // Three Ducal Ministers
    "太宰", "太傅", "太保", "太尉", "太師", "司空", "司徒", "丞相", // Inner court
    "侍中", "都督", "都護", "御史", // Special
    "國子", "秘書", "著作",
];

/// Build a regex fragment matching any title suffix or standalone title.
/// Sorted by length descending so longer patterns match first.
pub fn build_title_regex() -> String {
    let mut all: Vec<&str> = Vec::new();
    all.extend_from_slice(TITLE_SUFFIXES);
    all.extend_from_slice(STANDALONE_TITLES);

    // Sort by char-length descending for correct longest-match behavior
    all.sort_by_key(|b| std::cmp::Reverse(b.chars().count()));
    all.dedup();

    format!("(?:{})", all.join("|"))
}
