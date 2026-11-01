//! Link status, identity, candidates, remaps, folder matches and the search, over a fake
//! `LinkFiles` and (desktop) a temporary folder.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// Network (`\\host\s\a.png`), device (`\\.\COM1`) and URL (`https://h/a.png`) links are Refused, and the fake saw no call.
#[test]
#[ignore = "skeleton"]
fn a_status_is_missing_for_a_link_that_may_not_be_followed() {
    todo!()
}

// Same stamp: Ok without `identity`; new mtime, same fingerprint: Ok; new bytes: Modified; read error: Unreadable.
#[test]
#[ignore = "skeleton"]
fn a_touched_file_is_ok_and_a_changed_file_is_modified() {
    todo!()
}

// Same name, other bytes: not ok, `same_file == Some(false)`, a DifferentFile problem; the same file scores highest.
#[test]
#[ignore = "skeleton"]
fn a_different_file_with_the_same_name_is_refused_by_identity() {
    todo!()
}

// `fingerprint` of a 3 MiB file equals FilmCraft's for the same bytes; a 2 MiB file is hashed whole.
#[test]
#[ignore = "skeleton"]
fn the_fingerprint_reads_the_head_and_the_tail() {
    todo!()
}

// `derive_remap(C:\shoot\day1\a.mov, /Volumes/T7/shoot/day1/a.mov)` then `apply` maps `C:\shoot\day2\b.mov`; other prefixes give None.
#[test]
#[ignore = "skeleton"]
fn a_remap_carries_to_the_other_missing_links() {
    todo!()
}

// `folder_match` takes only a plain name: `a\b`, `file:%2Fx.png`, `C:x.png`, `NUL`, `..` give None; an extension swap works.
#[test]
#[ignore = "skeleton"]
fn relink_to_folder_matches_only_the_plain_name_in_that_folder() {
    todo!()
}

// A folder link loop is not followed; dot files are skipped; the limit sets `truncated`.
#[test]
#[ignore = "skeleton"]
fn the_search_never_follows_directory_links_and_stops_at_the_limit() {
    todo!()
}

// `cancelled` turning true mid-walk ends the run as Cancelled and no update follows it (task B5).
#[test]
#[ignore = "skeleton"]
fn a_cancelled_search_stops_and_reports_nothing_more() {
    todo!()
}
