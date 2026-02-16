/// Known compound (multi-character) surnames in the Six Dynasties period.
/// These must be checked BEFORE falling back to single-char surname.
pub const COMPOUND_SURNAMES: &[&str] = &[
    "司馬", "歐陽", "諸葛", "長孫", "令狐", "慕容", "拓跋", "宇文", "獨孤", "赫連", "呼延", "鮮于",
    "段幹", "公孫", "東方", "南宮", "西門", "上官", "夏侯", "皇甫", "尉遲", "澹臺", "公冶", "宗政",
    "濮陽", "淳于", "單于", "太叔", "申屠", "仲孫", "軒轅", "鍾離", "閭丘", "東郭", "南門", "壤駟",
    "禿髮", "宿勤",
];

/// Common single-character surnames attested in Six Dynasties historical texts.
/// This list covers the vast majority of persons appearing in 晉書/宋書/南齊書/梁書/陳書/魏書.
pub const SINGLE_SURNAMES: &[char] = &[
    // Top-frequency surnames across the Six Dynasties corpus
    '王', '李', '張', '劉', '陳', '楊', '趙', '黃', '周', '吳', '徐', '孫', '胡', '朱', '高', '林',
    '何', '郭', '馬', '羅', '梁', '宋', '鄭', '謝', '韓', '唐', '馮', '于', '董', '蕭', '程', '曹',
    '袁', '鄧', '許', '傅', '沈', '曾', '彭', '呂', '蘇', '盧', '蔣', '蔡', '賈', '丁', '魏', '薛',
    '葉', '閻', '余', '潘', '杜', '戴', '夏', '鍾', '汪', '田', '任', '姜', '范', '方', '石', '姚',
    '譚', '廖', '鄒', '熊', '金', '陸', '郝', '孔', '白', '崔', '康', '毛', '邱', '秦', '江', '史',
    '顧', '侯', '邵', '孟', '龍', '萬', '段', '雷', '錢', '湯', '尹', '黎', '易', '常', '武', '喬',
    '賀', '賴', '龔', '文', // Surnames especially prominent in Six Dynasties texts
    '庾', '桓', '殷', '荀', '裴', '虞', '褚', '柳', '阮', '嵇', '顏', '溫', '祖', '竇', '苻', '姬',
    '翟', '左', '伏', '卞', '鮑', '華', '廉', '管', '路', '嚴', '解', '耿', '宗', '甘', '臧', '樊',
    '和', '費', '甄', '辛', '雍', '蘭', '單', '穆', '成', '戚', '紀', '項', '祁', '毋', '牛', '邢',
    '滕', '鄔', '焦', '巴', '弓', '牧', '應', '苗', '明', '向', '鈕', '舒', '齊', '霍', '丘', '班',
    '仇', '游', '包', '盛', '房', '邊', '刁', '俞', '寇', '全', '符', '習', '岑', '封', '尚', '干',
    '暨', '居', '步', '都', '耿', '滿', '弘', '匡', '國', '聞', '索', '賁', '靳', '糜', '荊', '羊',
    '闞', '酈', '蒯', '種',
];

/// Regex character class for valid CJK name characters (excludes punctuation/whitespace).
const CJK_CHAR: &str = r"[^\s，。、；：！？「」『』（）〈〉《》【】\-]";

/// Build a regex fragment that matches any known full name (surname + 1-2 char given name).
/// Compound surnames are tried first (longer match), then single-char surnames.
/// `extra` can supply additional surnames discovered at runtime (e.g. from parsed persons).
pub fn build_name_regex(extra: &[String]) -> String {
    // Collect all compound surnames
    let mut compounds: Vec<&str> = COMPOUND_SURNAMES.to_vec();
    let mut singles: Vec<char> = SINGLE_SURNAMES.to_vec();

    for s in extra {
        let chars: Vec<char> = s.chars().collect();
        if chars.len() >= 2 {
            // Already in COMPOUND_SURNAMES?
            if !COMPOUND_SURNAMES.contains(&s.as_str()) {
                compounds.push(Box::leak(s.clone().into_boxed_str()));
            }
        } else if chars.len() == 1 && !singles.contains(&chars[0]) {
            singles.push(chars[0]);
        }
    }

    // Build compound part: (?:司馬|慕容|...){CJK}{1,2}
    let compound_alts: Vec<&str> = compounds.clone();
    let compound_part = if compound_alts.is_empty() {
        String::new()
    } else {
        format!("(?:{})", compound_alts.join("|"))
    };

    // Build single part: [王李張...]{CJK}{1,2}
    let single_chars: String = singles.iter().collect();
    let single_part = format!("[{}]", single_chars);

    let cjk = CJK_CHAR;
    // Combined: (?:(?:司馬|慕容|...){cjk}{1,2}|[王李張...]{cjk}{1,2})
    if compound_part.is_empty() {
        format!("{}{cjk}{{1,2}}", single_part)
    } else {
        format!(
            "(?:{}{cjk}{{1,2}}|{}{cjk}{{1,2}})",
            compound_part, single_part
        )
    }
}

/// Given a full name string (e.g. "褚淵", "司馬褧"), split into (surname, given_name).
/// Returns None if the string is too short.
pub fn split_name(full_name: &str) -> Option<(String, String)> {
    let chars: Vec<char> = full_name.chars().collect();
    if chars.len() < 2 {
        return None;
    }

    // Check compound surnames first
    for &cs in COMPOUND_SURNAMES {
        if full_name.starts_with(cs) {
            let cs_len = cs.chars().count();
            if chars.len() > cs_len {
                let given: String = chars[cs_len..].iter().collect();
                return Some((cs.to_string(), given));
            }
        }
    }

    // Single-character surname
    let surname = chars[0].to_string();
    let given: String = chars[1..].iter().collect();
    Some((surname, given))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_char_surname() {
        assert_eq!(split_name("褚淵"), Some(("褚".into(), "淵".into())));
        assert_eq!(split_name("韓秀"), Some(("韓".into(), "秀".into())));
    }

    #[test]
    fn test_compound_surname() {
        assert_eq!(split_name("司馬褧"), Some(("司馬".into(), "褧".into())));
        assert_eq!(split_name("禿髮烏孤"), Some(("禿髮".into(), "烏孤".into())));
    }

    #[test]
    fn test_two_char_given() {
        assert_eq!(split_name("柳世隆"), Some(("柳".into(), "世隆".into())));
    }
}
