//! Menu mnemonics (spec 9.7): the letter that opens a menu with Alt, or runs an entry of an open menu.
//!
//! A label's `&` marker, written in its words row (`&File`), names the letter. Without one the letter is the
//! label's first letter not yet used in the level, so a translated label needs no extra work. Letters are ASCII
//! letters and digits, the keys the keyboard reports as such, and upper case.

/// The label without its `&` marker, and the letter the first marker names. `&&` is a literal ampersand, a
/// marker before something other than an ASCII letter or digit names no letter, and later markers are dropped.
pub fn marked(label: &str) -> (String, Option<char>) {
    let (text, mark) = marked_at(label);
    (text, mark.map(|(letter, _)| letter))
}

/// [`marked`] with the character index of the marked letter in the returned text.
pub fn marked_at(label: &str) -> (String, Option<(char, usize)>) {
    let mut text = String::with_capacity(label.len());
    let mut mark = None;
    let mut at = 0;
    let mut chars = label.chars();
    while let Some(c) = chars.next() {
        if c != '&' {
            text.push(c);
            at += 1;
            continue;
        }
        let Some(next) = chars.next() else { break };
        if mark.is_none() && next != '&' && next.is_ascii_alphanumeric() {
            mark = Some((next.to_ascii_uppercase(), at));
        }
        text.push(next);
        at += 1;
    }
    (text, mark)
}

/// The mnemonic of each label in one menu level: the letter, upper case, and its character index in the label
/// without its markers. A `&` marker wins; a label whose marked letter an earlier label holds, and a label without a
/// marker, takes the first letter or digit of its text not yet used, trying the first character of each word before
/// the others.
pub fn assign(labels: &[&str]) -> Vec<Option<(char, usize)>> {
    let parsed: Vec<(String, Option<(char, usize)>)> = labels.iter().map(|label| marked_at(label)).collect();
    let mut used: Vec<char> = Vec::new();
    let mut out: Vec<Option<(char, usize)>> = Vec::with_capacity(parsed.len());
    for (_, mark) in &parsed {
        let held = mark.filter(|(letter, _)| !used.contains(letter));
        used.extend(held.map(|(letter, _)| letter));
        out.push(held);
    }
    for ((text, _), slot) in parsed.iter().zip(&mut out) {
        if slot.is_none() {
            *slot = first_unused(text, &used);
            used.extend(slot.map(|(letter, _)| letter));
        }
    }
    out
}

fn first_unused(text: &str, used: &[char]) -> Option<(char, usize)> {
    let mut previous_alnum = false;
    let mut starts = Vec::new();
    for (at, c) in text.chars().enumerate() {
        if c.is_alphanumeric() && !previous_alnum {
            starts.push((c, at));
        }
        previous_alnum = c.is_alphanumeric();
    }
    let all = text.chars().enumerate().map(|(at, c)| (c, at));
    starts
        .into_iter()
        .chain(all)
        .filter(|(c, _)| c.is_ascii_alphanumeric())
        .map(|(c, at)| (c.to_ascii_uppercase(), at))
        .find(|(c, _)| !used.contains(c))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn letters(labels: &[&str]) -> Vec<Option<char>> {
        assign(labels).into_iter().map(|mark| mark.map(|(letter, _)| letter)).collect()
    }

    #[test]
    fn auto_pick_prefers_a_word_start_not_used_in_the_level() {
        assert_eq!(letters(&["Copy", "Paste", "Arrange", "Print"]), [Some('C'), Some('P'), Some('A'), Some('R')]);
        assert_eq!(letters(&["Save", "Save As", "Share"]), [Some('S'), Some('A'), Some('H')]);
        assert_eq!(letters(&["Open", "Open Recent"]), [Some('O'), Some('R')]);
    }

    #[test]
    fn markers_win_and_reserve_their_letter() {
        assert_eq!(letters(&["Print", "P&aste", "Copy"]), [Some('P'), Some('A'), Some('C')]);
        assert_eq!(letters(&["Prin&t", "Paste"]), [Some('T'), Some('P')]);
        // Print comes first in the level, yet the marked letter is reserved before it picks.
        assert_eq!(letters(&["Print", "&Paste"]), [Some('R'), Some('P')]);
        assert_eq!(letters(&["&File", "&Fax"]), [Some('F'), Some('A')]);
    }

    #[test]
    fn a_label_without_a_typeable_letter_has_no_mnemonic() {
        assert_eq!(letters(&["文件", "Edit"]), [None, Some('E')]);
        assert_eq!(letters(&["文件(&F)", "Find"]), [Some('F'), Some('I')]);
    }

    #[test]
    fn the_index_is_the_character_of_the_letter_in_the_label_without_markers() {
        assert_eq!(assign(&["Save", "Save As"]), [Some(('S', 0)), Some(('A', 5))]);
        assert_eq!(assign(&["S&ave &As"]), [Some(('A', 1))]);
        assert_eq!(assign(&["Save && &Close"]), [Some(('C', 7))]);
        assert_eq!(assign(&["文件(&F)"]), [Some(('F', 3))]);
    }

    #[test]
    fn marked_strips_the_marker_and_keeps_a_literal_ampersand() {
        assert_eq!(marked("&File"), ("File".to_owned(), Some('F')));
        assert_eq!(marked("Save && Close"), ("Save & Close".to_owned(), None));
        assert_eq!(marked("S&ave &As"), ("Save As".to_owned(), Some('A')));
        assert_eq!(marked("Trailing&"), ("Trailing".to_owned(), None));
    }
}
