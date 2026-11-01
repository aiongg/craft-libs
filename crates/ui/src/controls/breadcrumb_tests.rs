//! `BreadcrumbBar` and path segments.
#![expect(clippy::todo, reason = "skeleton: removed when the bodies land")]

// `path_crumbs`: "/a/b" keys "/a" and "/a/b"; "C:\\x" keys "C:\\" and "C:\\x"; a UNC path keeps its leading "\\\\".
#[test]
#[ignore = "skeleton"]
fn path_segments_keep_the_paths_spelling() {
    todo!()
}

// In a harness: clicking an ancestor reports `opened`; the last segment is not a button.
#[test]
#[ignore = "skeleton"]
fn a_click_on_an_ancestor_opens_it_and_the_last_never_reports() {
    todo!()
}

// In a harness: a narrow bar shows the last segments and a "…" button whose menu opens a hidden one.
#[test]
#[ignore = "skeleton"]
fn segments_that_do_not_fit_go_behind_an_ellipsis() {
    todo!()
}
