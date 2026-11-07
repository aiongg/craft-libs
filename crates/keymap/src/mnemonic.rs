//! Menu mnemonics (spec 9.7): the letter that opens a menu with Alt, or runs an entry of an open menu.
//!
//! A label's `&` marker, written in its words row (`&File`), names the letter. Without one the letter is the
//! label's first letter not yet used in the level, so a translated label needs no extra work. Letters are ASCII
//! letters and digits, the keys the keyboard reports as such, and upper case.

/// The label without its `&` marker, and the letter the first marker names. `&&` is a literal ampersand, a
/// marker before something other than an ASCII letter or digit names no letter, and later markers are dropped.
pub fn marked(label: &str) -> (String, Option<char>) {
    let mut text = String::with_capacity(label.len());
    let mut letter = None;
    let mut chars = label.chars();
    while let Some(c) = chars.next() {
        if c != '&' {
            text.push(c);
            continue;
        }
        let Some(next) = chars.next() else { break };
        if letter.is_none() && next != '&' && next.is_ascii_alphanumeric() {
            letter = Some(next.to_ascii_uppercase());
        }
        text.push(next);
    }
    (text, letter)
}

/// The mnemonic of each label in one menu level. A `&` marker wins; a label whose marked letter an earlier label
/// holds, and a label without a marker, takes the first letter or digit of its text not yet used, trying the first
/// character of each word before the others. Letters are compared and returned in upper case.
pub fn assign(labels: &[&str]) -> Vec<Option<char>> {
    let parsed: Vec<(String, Option<char>)> = labels.iter().map(|label| marked(label)).collect();
    let mut used: Vec<char> = Vec::new();
    let mut out: Vec<Option<char>> = Vec::with_capacity(parsed.len());
    for (_, letter) in &parsed {
        let held = letter.filter(|letter| !used.contains(letter));
        used.extend(held);
        out.push(held);
    }
    for ((text, _), slot) in parsed.iter().zip(&mut out) {
        if slot.is_none() {
            *slot = first_unused(text, &used);
            used.extend(*slot);
        }
    }
    out
}

fn first_unused(text: &str, used: &[char]) -> Option<char> {
    let starts = text.split(|c: char| !c.is_alphanumeric()).filter_map(|word| word.chars().next());
    starts.chain(text.chars()).filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_uppercase()).find(|c| !used.contains(c))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn auto_pick_takes_the_first_letter_not_used_in_the_level() {
        assert_eq!(assign(&["Copy", "Paste", "Arrange", "Print"]), [Some('C'), Some('P'), Some('A'), Some('R')]);
        assert_eq!(assign(&["Save", "Save As", "Share"]), [Some('S'), Some('A'), Some('H')]);
    }

    #[test]
    fn a_word_start_comes_before_the_other_letters() {
        assert_eq!(assign(&["Open", "Open Recent"]), [Some('O'), Some('R')]);
    }

    #[test]
    fn a_marker_wins_and_reserves_its_letter_for_later_labels() {
        assert_eq!(assign(&["Print", "P&aste", "Copy"]), [Some('P'), Some('A'), Some('C')]);
        assert_eq!(assign(&["Prin&t", "Paste"]), [Some('T'), Some('P')]);
        // Print comes first in the level, yet the marked letter is reserved before it picks.
        assert_eq!(assign(&["Print", "&Paste"]), [Some('R'), Some('P')]);
    }

    #[test]
    fn a_marker_an_earlier_label_holds_falls_back_to_auto_pick() {
        assert_eq!(assign(&["&File", "&Fax"]), [Some('F'), Some('A')]);
    }

    #[test]
    fn a_label_without_a_typeable_letter_has_no_mnemonic() {
        assert_eq!(assign(&["文件", "Edit"]), [None, Some('E')]);
        assert_eq!(assign(&["文件(&F)", "Find"]), [Some('F'), Some('I')]);
    }

    #[test]
    fn marked_strips_the_marker_and_keeps_a_literal_ampersand() {
        assert_eq!(marked("&File"), ("File".to_owned(), Some('F')));
        assert_eq!(marked("Save && Close"), ("Save & Close".to_owned(), None));
        assert_eq!(marked("S&ave &As"), ("Save As".to_owned(), Some('A')));
        assert_eq!(marked("Trailing&"), ("Trailing".to_owned(), None));
    }
}
