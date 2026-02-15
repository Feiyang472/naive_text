/// Known compound (multi-character) surnames in the Six Dynasties period.
/// These must be checked BEFORE falling back to single-char surname.
pub const COMPOUND_SURNAMES: &[&str] = &[
    "司馬", "歐陽", "諸葛", "長孫", "令狐", "慕容", "拓跋", "宇文",
    "獨孤", "赫連", "呼延", "鮮于", "段幹", "公孫", "東方", "南宮",
    "西門", "上官", "夏侯", "皇甫", "尉遲", "澹臺", "公冶", "宗政",
    "濮陽", "淳于", "單于", "太叔", "申屠", "仲孫", "軒轅", "鍾離",
    "閭丘", "東郭", "南門", "壤駟", "禿髮",
];

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
        assert_eq!(
            split_name("褚淵"),
            Some(("褚".into(), "淵".into()))
        );
        assert_eq!(
            split_name("韓秀"),
            Some(("韓".into(), "秀".into()))
        );
    }

    #[test]
    fn test_compound_surname() {
        assert_eq!(
            split_name("司馬褧"),
            Some(("司馬".into(), "褧".into()))
        );
        assert_eq!(
            split_name("禿髮烏孤"),
            Some(("禿髮".into(), "烏孤".into()))
        );
    }

    #[test]
    fn test_two_char_given() {
        assert_eq!(
            split_name("柳世隆"),
            Some(("柳".into(), "世隆".into()))
        );
    }
}
