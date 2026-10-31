//! `craft_ui::text::elide`, measured in characters (one unit per char).
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// End keeps the start, Middle keeps a quarter at the end, Start keeps the end; text that fits is
// borrowed; a room narrower than "…" gives "…"; cuts never split a char.
#[test]
#[ignore = "skeleton"]
fn elide_cuts_where_asked_on_char_boundaries() {
    todo!()
}

// A right-to-left name cut at its end keeps its logical start ("لاشي مك…"); a left-to-right name
// in a right-to-left UI still loses its end.
#[test]
#[ignore = "skeleton"]
fn right_to_left_text_keeps_its_logical_start() {
    todo!()
}

// "/Users/me/Pictures/Library" drops folders to "…/Pictures/Library", a Windows path keeps "\",
// a trailing separator is not a name, a last name too long loses its end, "///" gives "…".
#[test]
#[ignore = "skeleton"]
fn paths_drop_leading_folders_first() {
    todo!()
}

// elide_chars(name, 80, Middle) of a 200-char name keeps 59 + "…" + 20 chars.
#[test]
#[ignore = "skeleton"]
fn char_limits_keep_start_and_end() {
    todo!()
}
