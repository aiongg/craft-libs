//! `craft_docpath::names`.

use crate::names::{CopyNumbering, MAX_NAME_BYTES, natural_cmp, safe_file_name, safe_file_stem, unique_file_name};

// Reserved and control characters become `_`, bidi controls go, trailing dots and spaces go,
// "NUL.txt" becomes "_NUL.txt", a 300-byte name is cut to 200 bytes on a char boundary keeping
// ".pdf", and "..." gives the fallback.
#[test]
fn user_text_becomes_a_safe_file_name() {
    assert_eq!(safe_file_name("Q3: plan?.pdf", "x"), "Q3_ plan_.pdf");
    assert_eq!(safe_file_name("a\tb/c", "x"), "a_b_c");
    assert_eq!(safe_file_name("  report. . ", "x"), "report");
    assert_eq!(safe_file_name("image\u{202e}gnp.exe", "x"), "imagegnp.exe");
    assert_eq!(safe_file_name("NUL.txt", "x"), "_NUL.txt");
    assert_eq!(safe_file_name("...", "Untitled"), "Untitled");
    let long = format!("{}.pdf", "é".repeat(150));
    let cut = safe_file_name(&long, "x");
    assert!(cut.len() <= MAX_NAME_BYTES && cut.ends_with("é.pdf"), "{cut}");
    assert!(safe_file_name(&"a".repeat(300), "x").len() == MAX_NAME_BYTES);
}

// "Report.PDF" with extension "pdf" is "Report"; "COM1" is "_COM1"; "  " is empty.
#[test]
fn stems_drop_the_extension_the_app_adds() {
    assert_eq!(safe_file_stem("Report.PDF", "pdf"), "Report");
    assert_eq!(safe_file_stem("Report.pdf.pdf", "pdf"), "Report.pdf");
    assert_eq!(safe_file_stem("COM1", "pdf"), "_COM1");
    assert_eq!(safe_file_stem("  ", "pdf"), "");
    assert_eq!(safe_file_stem(".pdf", "pdf"), "");
}

// "a.txt", "a 2.txt" taken: Space gives "a 3.txt", Parenthesized "a (2).txt" when only "a.txt"
// is taken; "photo (2).jpg" counts on to "photo (3).jpg"; a name with no extension numbers its end.
#[test]
fn taken_names_are_numbered_before_the_extension() {
    let taken = |names: &'static [&'static str]| move |n: &str| names.contains(&n);
    assert_eq!(unique_file_name("a.txt", CopyNumbering::Space, taken(&["a.txt", "a 2.txt"])), "a 3.txt");
    assert_eq!(unique_file_name("a.txt", CopyNumbering::Parenthesized, taken(&["a.txt"])), "a (2).txt");
    assert_eq!(unique_file_name("a.txt", CopyNumbering::Parenthesized, taken(&[])), "a.txt");
    assert_eq!(unique_file_name("photo (2).jpg", CopyNumbering::Parenthesized, taken(&["photo (2).jpg"])), "photo (3).jpg");
    assert_eq!(unique_file_name("notes", CopyNumbering::Parenthesized, taken(&["notes"])), "notes (2)");
    assert_eq!(unique_file_name("a.tar.gz", CopyNumbering::Space, taken(&["a.tar.gz"])), "a.tar 2.gz");
}

// Space style (Finder) never counts on a number that is already in the name: the number goes
// after the whole stem.
#[test]
fn space_numbering_keeps_a_number_in_the_name() {
    let taken = |names: &'static [&'static str]| move |n: &str| names.contains(&n);
    assert_eq!(unique_file_name("Invoice 2026.pdf", CopyNumbering::Space, taken(&["Invoice 2026.pdf"])), "Invoice 2026 2.pdf");
    assert_eq!(unique_file_name("Chapter 1.docx", CopyNumbering::Space, taken(&["Chapter 1.docx", "Chapter 1 2.docx"])), "Chapter 1 3.docx");
    assert_eq!(unique_file_name("photo 2.jpg", CopyNumbering::Space, taken(&["photo 2.jpg"])), "photo 2 2.jpg");
}

// "img2" sorts before "img10", "IMG3" between them ignoring case, "a01" beside "a1" by length.
#[test]
fn natural_order_compares_digit_runs_by_value() {
    let mut names = ["img10", "img2", "IMG3", "a1", "a01"];
    names.sort_by(|a, b| natural_cmp(a, b));
    assert_eq!(names, ["a01", "a1", "img2", "IMG3", "img10"]);
}
